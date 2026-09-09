use protos::{Boundary, Enclosure, Protos, Protosizable, Textualizable};
#[test]
fn structural_forms_keep_their_own_extents() {
 let form = "Reviewer.{ 2024 17 }".protosize().expect("structure");
 assert_eq!(form.textualize(), "Reviewer.{ 2024 17 }");
 let Protos::Headed { body, .. } = form else { panic!("headed") };
 let Protos::Enclosed { enclosure, children, .. } = *body else { panic!("enclosed") };
 assert_eq!(enclosure, Enclosure::Braced); assert_eq!(children.len(), 2);
}
#[test]
fn opaque_guillemets_escape_their_closer() {
 let form = "«she said \\»no\\» and left»".protosize().expect("string");
 assert_eq!(form.textualize(), "«she said \\»no\\» and left»");
 assert!(matches!(form, Protos::Opaque { boundary: Boundary::Guillemets, .. }));
}
