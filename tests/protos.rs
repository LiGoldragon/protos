use protos::{
    Boundary, BoundedProtosizable, Enclosure, Protos, Protosizable, ReaderBudget, Textualizable,
};
use std::process::Command;
#[test]
fn structural_forms_keep_their_own_extents() {
    let form = "Reviewer.{ 2024 17 }".protosize().expect("structure");
    assert_eq!(form.textualize(), "Reviewer.{ 2024 17 }");
    let Protos::Headed { body, .. } = &form else {
        panic!("headed")
    };
    let Protos::Enclosed {
        enclosure,
        children,
        ..
    } = body.as_ref()
    else {
        panic!("enclosed")
    };
    assert_eq!(*enclosure, Enclosure::Braced);
    assert_eq!(children.len(), 2);
}
#[test]
fn opaque_guillemets_escape_their_closer() {
    let form = "«she said \\»no\\» and left»".protosize().expect("string");
    assert_eq!(form.textualize(), "«she said \\»no\\» and left»");
    assert!(matches!(
        form,
        Protos::Opaque {
            boundary: Boundary::Guillemets,
            ..
        }
    ));
}

#[test]
fn a_guillemet_escape_does_not_consume_an_ordinary_backslash() {
    let form = "«path\\segment»".protosize().expect("string");
    assert_eq!(form.textualize(), "«path\\segment»");
}

#[test]
fn parentheses_are_one_balanced_opaque_structure() {
    let form = "(a (b))".protosize().expect("meaning structure");
    assert_eq!(form.textualize(), "(a (b))");
    assert!(matches!(
        form,
        Protos::Opaque {
            boundary: Boundary::Parentheses,
            ..
        }
    ));
}

#[test]
fn parentheses_escape_their_structural_glyphs() {
    for text in ["(x\\))", "(\\()"] {
        let form = text.protosize().expect("escaped meaning");
        assert_eq!(form.textualize(), text);
    }
}

#[test]
fn a_meaning_keeps_an_ordinary_backslash() {
    let form = "(path\\segment)".protosize().expect("meaning");
    assert_eq!(form.textualize(), "(path\\segment)");
    assert!(matches!(
        &form,
        Protos::Opaque { content, .. } if content == "path\\segment"
    ));
}

#[test]
fn reader_budget_bounds_recursive_descent() {
    let mut budget = ReaderBudget { remaining: 2 };
    assert!("{ { Ada } }".protosize_with(&mut budget).is_err());
}

#[test]
fn bounded_depth_probe_child() {
    if std::env::var_os("PROTOS_DEPTH_PROBE").is_none() {
        return;
    }
    let text = format!("{}Ada{}", "{".repeat(100_001), "}".repeat(100_001));
    let mut budget = ReaderBudget { remaining: 100_002 };
    assert!(text.protosize_with(&mut budget).is_err());
}

#[test]
fn depth_is_bounded_independently_of_node_budget() {
    let text = format!("[ {}]", "Ada ".repeat(10_000));
    let mut budget = ReaderBudget { remaining: 10_001 };
    let form = text.protosize_with(&mut budget).expect("wide structure");
    assert!(matches!(&form, Protos::Enclosed { children, .. } if children.len() == 10_000));
}

#[test]
fn deeply_constructed_forms_print_without_recursion() {
    let mut form = Protos::Bare {
        extent: protos::Extent { start: 0, end: 1 },
        text: String::from("Ada"),
    };
    for _ in 0..100_001 {
        form = Protos::Headed {
            extent: protos::Extent { start: 0, end: 1 },
            head: protos::Symbol(String::from("A")),
            constraints: None,
            separator: protos::Separator::Period,
            body: Box::new(form),
        };
    }
    let text = form.textualize();
    assert_eq!(text.len(), 200_005);
    assert!(text.ends_with("Ada"));
    drop(form);
}

#[test]
fn deeply_constructed_forms_drop_without_recursion() {
    let mut form = Protos::Bare {
        extent: protos::Extent { start: 0, end: 1 },
        text: String::from("Ada"),
    };
    for _ in 0..100_000 {
        form = Protos::Headed {
            extent: protos::Extent { start: 0, end: 1 },
            head: protos::Symbol(String::from("A")),
            constraints: None,
            separator: protos::Separator::Period,
            body: Box::new(form),
        };
    }
    drop(form);
}

#[test]
fn depth_probe_is_bounded_by_memory_and_time() {
    let executable = std::env::current_exe().expect("test executable");
    let output = Command::new("sh")
        .arg("-c")
        .arg("ulimit -v 262144; exec timeout 30 \"$0\" \"$@\"")
        .arg(executable)
        .arg("--exact")
        .arg("bounded_depth_probe_child")
        .arg("--nocapture")
        .env("PROTOS_DEPTH_PROBE", "1")
        .output()
        .expect("bounded child process");
    assert!(
        output.status.success(),
        "deep reader probe failed: {}\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn angle_brackets_remain_structural() {
    let form = "<Thing Other>".protosize().expect("structural angle");
    assert_eq!(form.textualize(), "<Thing Other>");
    assert!(matches!(
        form,
        Protos::Enclosed {
            enclosure: Enclosure::Angled,
            ..
        }
    ));
}

#[test]
fn qualified_names_are_one_structural_form() {
    let text = "Processable<[Clonable Sendable] Serializable>.[ Vector<String> ]";
    let form = text.protosize().expect("qualified headed form");
    assert_eq!(
        form.textualize(),
        "Processable<[ Clonable Sendable ] Serializable>.[ Vector <String> ]"
    );
    let Protos::Headed {
        head,
        constraints: Some(constraints),
        body,
        ..
    } = &form
    else {
        panic!("qualified heading")
    };
    assert_eq!(head.0, "Processable");
    assert!(matches!(
        constraints.as_ref(),
        Protos::Enclosed { enclosure: Enclosure::Angled, children, .. }
        if children.len() == 2
    ));
    assert!(matches!(
        body.as_ref(),
        Protos::Enclosed { children, .. }
        if matches!(children.as_slice(), [Protos::Bare { text, .. }, Protos::Enclosed { enclosure: Enclosure::Angled, .. }] if text == "Vector")
    ));
}
