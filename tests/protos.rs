use protos::{Boundary, Enclosure, Protos, Protosizable, Textualizable};
#[test]
fn structural_forms_keep_their_own_extents() {
    let form = "Reviewer.{ 2024 17 }".protosize().expect("structure");
    assert_eq!(form.textualize(), "Reviewer.{ 2024 17 }");
    let Protos::Headed { body, .. } = form else {
        panic!("headed")
    };
    let Protos::Enclosed {
        enclosure,
        children,
        ..
    } = *body
    else {
        panic!("enclosed")
    };
    assert_eq!(enclosure, Enclosure::Braced);
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
