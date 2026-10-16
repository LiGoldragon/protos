//! The canonical print expands vertically: a structure with more than one
//! element, one of which has a next layer, opens on its line and its elements
//! hang beneath the first, aligned; the closing delimiter ends the last
//! element's line. Elements that are all leaves sit on one line, and an angled
//! enclosure stays tight against the element it follows. The text is the
//! product here, so the text is what is asserted; every expected text is
//! written out by hand.

use protos::{Canonicalizable, Compactable, Extent, Protos, Protosizable, Textualizable};

/// Read the text and print it canonically, holding the print to the reading:
/// the print reads back as the read structure canonicalized, extents and all,
/// so the extents canonicalization assigns are those of this very text.
fn reprint(text: &str) -> String {
    let mut read = text
        .protosize()
        .unwrap_or_else(|error| panic!("{text:?} reads: {error}"));
    let printed = read.textualize();
    let reread = printed
        .protosize()
        .unwrap_or_else(|error| panic!("{printed:?} reads back: {error}"));
    read.canonicalize();
    assert_eq!(reread, read, "{printed:?} reads back as another structure");
    printed
}

#[test]
fn leaves_sit_on_one_line() {
    assert_eq!(reprint("{ a b }"), "{ a b }");
    assert_eq!(reprint("[ 0 42 -42 ]"), "[ 0 42 -42 ]");
    assert_eq!(reprint("[ «a b» Pending ]"), "[ «a b» Pending ]");
    assert_eq!(reprint("{}"), "{}");
}

#[test]
fn one_element_with_a_next_layer_stays_with_its_opener() {
    assert_eq!(
        reprint("[ flow:[ FlowId Voice Event ] ]"),
        "[ flow:[ FlowId Voice Event ] ]"
    );
    assert_eq!(reprint("Observed.Locks.[]"), "Observed.Locks.[]");
}

#[test]
fn several_elements_with_a_next_layer_hang_aligned_beneath_the_first() {
    assert_eq!(
        reprint("Locked.{ 442 MyLock 6329f1 [ /abs/path ] «why I hold it» }"),
        "Locked.{ 442\n         MyLock\n         6329f1\n         [ /abs/path ]\n         «why I hold it» }"
    );
}

#[test]
fn nested_structures_hang_at_their_own_column() {
    assert_eq!(
        reprint("[ Launch.{ Voice Brief.String } Report.{ FlowId Event } ]"),
        "[ Launch.{ Voice\n           Brief.String }\n  Report.{ FlowId Event } ]"
    );
}

#[test]
fn an_angled_enclosure_stays_tight_against_the_element_before_it() {
    assert_eq!(
        reprint("[ Vector<Event> Result<String Integer> ]"),
        "[ Vector<Event> Result<String Integer> ]"
    );
    assert_eq!(reprint("[ a. <b> ]"), "[ a.<b> ]");
    assert_eq!(
        reprint("[ Generated.Vector<String> Checked.String ]"),
        "[ Generated.Vector<String>\n  Checked.String ]"
    );
    assert_eq!(
        reprint("{ FlowId State.[ Running Ended ] Vector<Event> }"),
        "{ FlowId\n  State.[ Running Ended ]\n  Vector<Event> }"
    );
}

#[test]
fn a_string_that_spans_lines_hangs_from_the_column_it_leaves() {
    assert_eq!(reprint("{ «one\ntwo» [ a ] }"), "{ «one\ntwo»\n  [ a ] }");
}

#[test]
fn the_compact_print_writes_every_structure_on_one_line() {
    let read = "Locked.{ 442\n MyLock [ /abs/path ]\n «why I hold it» }"
        .protosize()
        .expect("reads");
    assert_eq!(
        read.compact(),
        "Locked.{ 442 MyLock [ /abs/path ] «why I hold it» }"
    );
}

#[test]
fn canonical_extents_are_those_of_the_vertical_print() {
    let mut read = "{ a [ b ] }".protosize().expect("reads");
    read.canonicalize();
    let Protos::Enclosed {
        extent, children, ..
    } = &read
    else {
        panic!("an enclosure");
    };
    assert_eq!(*extent, Extent { start: 0, end: 13 });
    assert_eq!(protos_extent(&children[1]), Extent { start: 6, end: 11 });
}

fn protos_extent(node: &Protos) -> Extent {
    match node {
        Protos::Headed { extent, .. }
        | Protos::Enclosed { extent, .. }
        | Protos::Opaque { extent, .. }
        | Protos::Bare { extent, .. } => *extent,
    }
}
