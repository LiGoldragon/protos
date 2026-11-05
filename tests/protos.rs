use protos::{
    Boundary, BoundedProtosizable, BuildsReaderBudget, Canonicalizable, Enclosure, Extent,
    LexicallyReadable, LexicallyTraversable, Protos, ReaderBudget, ReaderBudgetConstructionReason,
    ReaderBudgetField, ReadsReaderBudget, Separator, Spendable, Symbol, Textualizable,
};
use std::process::Command;

trait ReadsFixture {
    fn protosize(&self) -> Result<Protos, protos::Error>;
}
impl<T: BoundedProtosizable + ?Sized> ReadsFixture for T {
    fn protosize(&self) -> Result<Protos, protos::Error> {
        let mut budget = ReaderBudget::from_native_limits(4_096, 256).expect("fixture budget");
        self.protosize_with(&mut budget)
    }
}

fn form_extent(form: &Protos) -> Extent {
    match form {
        Protos::Headed { extent, .. }
        | Protos::Enclosed { extent, .. }
        | Protos::Opaque { extent, .. }
        | Protos::Bare { extent, .. } => *extent,
    }
}
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
fn invalid_separator_chains_remain_one_bare_run() {
    for text in [
        "a.",
        ".a",
        "a..b",
        "a.b.",
        "..",
        ".codex/agents/worker.toml",
        ".猫/龍.",
    ] {
        let form = text.protosize().expect("bare run");
        assert!(
            matches!(&form, Protos::Bare { text: value, .. } if value == text),
            "{text}: {form:?}"
        );
        assert_eq!(form.textualize(), text);
    }
}

#[test]
fn complete_separator_chains_remain_headed_with_exact_extents() {
    for text in [
        "lower.case",
        "Upper.Case",
        "a:b:c",
        "a!b!c",
        "https://example.org/a",
        "猫.龍",
        "Observed.Locks.[]",
        "Reviewer.{ 2024 17 }",
        "Some.(x)",
    ] {
        let form = text.protosize().expect("headed structure");
        assert!(matches!(&form, Protos::Headed { .. }), "{text}: {form:?}");
        assert_eq!(
            form_extent(&form),
            Extent {
                start: 0,
                end: text.len()
            }
        );
        assert_eq!(form.textualize(), text);
    }
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
    let mut budget = ReaderBudget::from_native_limits(2, 256).expect("fixture budget");
    assert!("{ { Ada } }".protosize_with(&mut budget).is_err());
}

#[test]
fn reader_budget_construction_checks_each_field_and_zero_exhausts_safely() {
    let mut zero = ReaderBudget::from_integer_limits(0, 1).expect("zero remaining is valid");
    assert!(!zero.spend());
    assert_eq!(zero.remaining(), 0);
    assert_eq!(zero.maximum_depth(), 1);

    let negative = ReaderBudget::from_integer_limits(1, -1).expect_err("negative depth");
    assert_eq!(negative.field, ReaderBudgetField::MaximumDepth);
    assert_eq!(negative.reason, ReaderBudgetConstructionReason::Negative);

    let negative_remaining =
        ReaderBudget::from_integer_limits(-1, 1).expect_err("negative remaining");
    assert_eq!(negative_remaining.field, ReaderBudgetField::Remaining);
    assert_eq!(
        negative_remaining.reason,
        ReaderBudgetConstructionReason::Negative
    );
}

#[test]
fn reader_budget_checks_widths_and_preserves_the_shared_maximum() {
    let shared_maximum = usize::try_from(i64::MAX).unwrap_or(usize::MAX);
    let budget = ReaderBudget::from_native_limits(shared_maximum, shared_maximum)
        .expect("largest value shared by native and integer widths");
    assert_eq!(budget.remaining(), shared_maximum as i64);
    assert_eq!(budget.maximum_depth(), shared_maximum as i64);

    #[cfg(target_pointer_width = "64")]
    for (remaining, maximum_depth, field) in [
        (usize::MAX, 1, ReaderBudgetField::Remaining),
        (1, usize::MAX, ReaderBudgetField::MaximumDepth),
    ] {
        let error = ReaderBudget::from_native_limits(remaining, maximum_depth)
            .expect_err("usize value outside the integer width");
        assert_eq!(error.field, field);
        assert_eq!(
            error.reason,
            ReaderBudgetConstructionReason::OutsideIntegerWidth
        );
    }

    #[cfg(target_pointer_width = "32")]
    for (remaining, maximum_depth, field) in [
        (i64::from(u32::MAX) + 1, 1, ReaderBudgetField::Remaining),
        (1, i64::from(u32::MAX) + 1, ReaderBudgetField::MaximumDepth),
    ] {
        let error = ReaderBudget::from_integer_limits(remaining, maximum_depth)
            .expect_err("integer value outside the native width");
        assert_eq!(error.field, field);
        assert_eq!(
            error.reason,
            ReaderBudgetConstructionReason::OutsideNativeWidth
        );
    }
}

#[test]
fn reader_budget_depth_boundary_and_consumption_are_exact() {
    let mut budget = ReaderBudget::from_native_limits(1, 3).expect("fixture budget");
    assert!(budget.permits_depth(2));
    assert!(!budget.permits_depth(3));
    assert!(!budget.permits_depth(usize::MAX));
    assert!(budget.spend());
    assert_eq!(budget.remaining(), 0);
    assert_eq!(budget.maximum_depth(), 3);
    assert!(!budget.spend());
}

#[test]
fn lexical_cursor_uses_the_readers_space_comment_and_token_boundaries() {
    let mut cursor = " ; comment\n { 65536 next }".lexical_cursor();
    cursor.expect_delimiter('{').expect("opening delimiter");
    assert_eq!(cursor.token().expect("first token"), "65536");
    assert_eq!(cursor.token().expect("second token"), "next");
    cursor.expect_delimiter('}').expect("closing delimiter");
    assert_eq!(cursor.offset(), " ; comment\n { 65536 next }".len());
}

#[test]
fn lexical_cursor_counts_unicode_space_and_tokens_in_utf8_bytes() {
    let text = "\u{2003}\u{3000}; 猫\n 龍.虎 }";
    let mut cursor = text.lexical_cursor();
    assert_eq!(cursor.token().expect("utf8 token"), "龍.虎");
    assert_eq!(
        cursor.offset(),
        text.find("龍.虎").expect("utf8 token start") + "龍.虎".len()
    );
    let error = cursor.token().expect_err("delimiter is not a token");
    let delimiter = text.find('}').expect("closing delimiter");
    assert_eq!(
        error.extent,
        Extent {
            start: delimiter,
            end: delimiter
        }
    );
    cursor
        .expect_delimiter('}')
        .expect("utf8-positioned delimiter");
    assert_eq!(cursor.offset(), text.len());
}

#[test]
fn lexical_cursor_accepts_only_the_structural_delimiters() {
    for delimiter in ['{', '}', '[', ']', '<', '>'] {
        let text = delimiter.to_string();
        let mut cursor = text.as_str().lexical_cursor();
        cursor
            .expect_delimiter(delimiter)
            .expect("accepted delimiter");
        assert_eq!(cursor.offset(), delimiter.len_utf8());
    }

    for delimiter in ['(', ')', '«', '»', ';', '.'] {
        let mut cursor = "".lexical_cursor();
        let error = cursor
            .expect_delimiter(delimiter)
            .expect_err("nonstructural delimiter");
        assert_eq!(error.extent, Extent { start: 0, end: 0 });
    }
}

#[test]
fn lexical_cursor_stops_tokens_at_every_lexical_boundary() {
    for boundary in [
        " ", "\n", "{", "}", "[", "]", "<", ">", "«", "»", "(", ")", ";",
    ] {
        let text = format!("token{boundary}next");
        let mut cursor = text.as_str().lexical_cursor();
        assert_eq!(cursor.token().expect("token"), "token", "{boundary:?}");
        assert_eq!(cursor.offset(), "token".len(), "{boundary:?}");
    }
}

#[test]
fn bounded_depth_probe_child() {
    if std::env::var_os("PROTOS_DEPTH_PROBE").is_none() {
        return;
    }
    let text = format!("{}Ada{}", "{".repeat(100_001), "}".repeat(100_001));
    let mut budget = ReaderBudget::from_native_limits(100_002, 256).expect("fixture budget");
    assert!(text.protosize_with(&mut budget).is_err());
}

#[test]
fn depth_is_bounded_independently_of_node_budget() {
    let text = format!("[ {}]", "Ada ".repeat(10_000));
    let mut budget = ReaderBudget::from_native_limits(10_001, 256).expect("fixture budget");
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
fn canonicalization_assigns_utf8_extents_without_reading() {
    let mut form = Protos::Headed {
        extent: Extent { start: 99, end: 99 },
        head: Symbol("H".into()),
        constraints: Some(Box::new(Protos::Enclosed {
            extent: Extent { start: 99, end: 99 },
            enclosure: Enclosure::Angled,
            children: vec![
                Protos::Bare {
                    extent: Extent { start: 99, end: 99 },
                    text: "A".into(),
                },
                Protos::Bare {
                    extent: Extent { start: 99, end: 99 },
                    text: "B".into(),
                },
            ],
        })),
        separator: Separator::Period,
        body: Box::new(Protos::Enclosed {
            extent: Extent { start: 99, end: 99 },
            enclosure: Enclosure::Braced,
            children: vec![
                Protos::Opaque {
                    extent: Extent { start: 99, end: 99 },
                    boundary: Boundary::Guillemets,
                    content: "é»".into(),
                },
                Protos::Opaque {
                    extent: Extent { start: 99, end: 99 },
                    boundary: Boundary::Parentheses,
                    content: "x)".into(),
                },
                Protos::Enclosed {
                    extent: Extent { start: 99, end: 99 },
                    enclosure: Enclosure::Bracketed,
                    children: vec![],
                },
            ],
        }),
    };
    form.canonicalize();
    let text = form.textualize();
    assert_eq!(text, "H<A B>.{ «é\\»» (x\\)) [] }");
    let Protos::Headed {
        extent,
        constraints: Some(constraints),
        body,
        ..
    } = &form
    else {
        panic!("headed form")
    };
    assert_eq!(&text[extent.start..extent.end], text);
    let constraints_extent = form_extent(constraints);
    assert_eq!(
        &text[constraints_extent.start..constraints_extent.end],
        "<A B>"
    );
    let Protos::Enclosed {
        extent, children, ..
    } = body.as_ref()
    else {
        panic!("braced body")
    };
    assert_eq!(&text[extent.start..extent.end], "{ «é\\»» (x\\)) [] }");
    for (child, expected) in children.iter().zip(["«é\\»»", "(x\\))", "[]"]) {
        let child_extent = form_extent(child);
        assert_eq!(&text[child_extent.start..child_extent.end], expected);
    }
}

#[test]
fn canonicalization_is_iterative_for_wide_and_deep_structures() {
    let mut wide = Protos::Enclosed {
        extent: Extent { start: 0, end: 0 },
        enclosure: Enclosure::Bracketed,
        children: (0..5_000)
            .map(|_| Protos::Bare {
                extent: Extent { start: 0, end: 0 },
                text: "é".into(),
            })
            .collect(),
    };
    wide.canonicalize();
    let wide_text = wide.textualize();
    assert_eq!(form_extent(&wide).end, wide_text.len());

    let mut deep = Protos::Bare {
        extent: Extent { start: 0, end: 0 },
        text: "x".into(),
    };
    for _ in 0..100_000 {
        deep = Protos::Headed {
            extent: Extent { start: 0, end: 0 },
            head: Symbol("V".into()),
            constraints: None,
            separator: Separator::Period,
            body: Box::new(deep),
        };
    }
    deep.canonicalize();
    assert_eq!(
        form_extent(&deep),
        Extent {
            start: 0,
            end: 200_001
        }
    );
    drop(deep);
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
        "Processable<[ Clonable Sendable ] Serializable>.[ Vector<String> ]"
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
