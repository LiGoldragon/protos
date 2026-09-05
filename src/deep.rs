//! Iterative ordinary operations for the recursive structural anatomy.

use std::fmt;

use crate::anatomy::{Delineation, Enclosure, Head, Protoform, Separator, Situated, Situation};

enum CopyJob<'a> {
    Form(&'a Protoform),
    Head(&'a Head),
    Headed(Separator),
    Enclosed(Enclosure, usize),
    HeadQualified(crate::Symbol, usize),
    FormQualified(crate::Symbol, usize),
}

enum Value {
    Form(Protoform),
    Head(Head),
}

struct Copier<'a> {
    work: Vec<CopyJob<'a>>,
    values: Vec<Value>,
}

trait Copying<'a> {
    fn head(value: Value) -> Head;
    fn form(value: Value) -> Protoform;
    fn forms(&mut self, count: usize) -> Vec<Protoform>;
    fn copy(self) -> Value;
    fn head_root(head: &'a Head) -> Head;
    fn form_root(form: &'a Protoform) -> Protoform;
}

impl<'a> Copying<'a> for Copier<'a> {
    fn head(value: Value) -> Head {
        match value {
            Value::Head(head) => head,
            Value::Form(_) => unreachable!("the clone worklist preserves its anatomy"),
        }
    }
    fn form(value: Value) -> Protoform {
        match value {
            Value::Form(form) => form,
            Value::Head(_) => unreachable!("the clone worklist preserves its anatomy"),
        }
    }
    fn forms(&mut self, count: usize) -> Vec<Protoform> {
        self.values
            .split_off(self.values.len() - count)
            .into_iter()
            .map(Self::form)
            .collect()
    }
    fn copy(mut self) -> Value {
        while let Some(job) = self.work.pop() {
            match job {
                CopyJob::Form(Protoform::Headed(head, separator, body)) => {
                    self.work.push(CopyJob::Headed(*separator));
                    self.work.push(CopyJob::Form(body));
                    self.work.push(CopyJob::Head(head));
                }
                CopyJob::Form(Protoform::Enclosed(enclosure, children)) => {
                    self.work
                        .push(CopyJob::Enclosed(*enclosure, children.len()));
                    self.work.extend(children.iter().rev().map(CopyJob::Form));
                }
                CopyJob::Form(Protoform::Quoted(text)) => self
                    .values
                    .push(Value::Form(Protoform::Quoted(text.clone()))),
                CopyJob::Form(Protoform::Parenthesized(opaque)) => self
                    .values
                    .push(Value::Form(Protoform::Parenthesized(opaque.clone()))),
                CopyJob::Form(Protoform::Bare(bare)) => {
                    self.values.push(Value::Form(Protoform::Bare(bare.clone())))
                }
                CopyJob::Form(Protoform::Qualified(symbol, constraints)) => {
                    self.work
                        .push(CopyJob::FormQualified(symbol.clone(), constraints.len()));
                    self.work
                        .extend(constraints.iter().rev().map(CopyJob::Form));
                }
                CopyJob::Head(Head::Symbol(symbol)) => {
                    self.values.push(Value::Head(Head::Symbol(symbol.clone())))
                }
                CopyJob::Head(Head::Qualified(symbol, constraints)) => {
                    self.work
                        .push(CopyJob::HeadQualified(symbol.clone(), constraints.len()));
                    self.work
                        .extend(constraints.iter().rev().map(CopyJob::Form));
                }
                CopyJob::Headed(separator) => {
                    let body = Self::form(self.values.pop().expect("headed body"));
                    let head = Self::head(self.values.pop().expect("headed head"));
                    self.values.push(Value::Form(Protoform::Headed(
                        head,
                        separator,
                        Box::new(body),
                    )));
                }
                CopyJob::Enclosed(enclosure, count) => {
                    let forms = self.forms(count);
                    self.values
                        .push(Value::Form(Protoform::Enclosed(enclosure, forms)));
                }
                CopyJob::HeadQualified(symbol, count) => {
                    let forms = self.forms(count);
                    self.values
                        .push(Value::Head(Head::Qualified(symbol, forms)));
                }
                CopyJob::FormQualified(symbol, count) => {
                    let forms = self.forms(count);
                    self.values
                        .push(Value::Form(Protoform::Qualified(symbol, forms)));
                }
            }
        }
        self.values.pop().expect("one copied root")
    }
    fn head_root(head: &'a Head) -> Head {
        Self::head(
            Self {
                work: vec![CopyJob::Head(head)],
                values: Vec::new(),
            }
            .copy(),
        )
    }
    fn form_root(form: &'a Protoform) -> Protoform {
        Self::form(
            Self {
                work: vec![CopyJob::Form(form)],
                values: Vec::new(),
            }
            .copy(),
        )
    }
}

impl Clone for Head {
    fn clone(&self) -> Self {
        Copier::head_root(self)
    }
}

impl Clone for Protoform {
    fn clone(&self) -> Self {
        Copier::form_root(self)
    }
}

enum Compare<'a> {
    Form(&'a Protoform, &'a Protoform),
    Head(&'a Head, &'a Head),
}

struct Comparer<'a> {
    work: Vec<Compare<'a>>,
}
trait Comparing<'a> {
    fn same(self) -> bool;
    fn heads(left: &'a Head, right: &'a Head) -> bool;
    fn forms(left: &'a Protoform, right: &'a Protoform) -> bool;
}
impl<'a> Comparing<'a> for Comparer<'a> {
    fn same(mut self) -> bool {
        while let Some(job) = self.work.pop() {
            match job {
                Compare::Form(
                    Protoform::Headed(left_head, left_separator, left_body),
                    Protoform::Headed(right_head, right_separator, right_body),
                ) => {
                    if left_separator != right_separator {
                        return false;
                    }
                    self.work.push(Compare::Form(left_body, right_body));
                    self.work.push(Compare::Head(left_head, right_head));
                }
                Compare::Form(
                    Protoform::Enclosed(left_enclosure, left_children),
                    Protoform::Enclosed(right_enclosure, right_children),
                ) => {
                    if left_enclosure != right_enclosure
                        || left_children.len() != right_children.len()
                    {
                        return false;
                    }
                    self.work.extend(
                        left_children
                            .iter()
                            .zip(right_children)
                            .map(|(left, right)| Compare::Form(left, right)),
                    );
                }
                Compare::Form(Protoform::Quoted(left_opaque), Protoform::Quoted(right_opaque)) => {
                    if left_opaque != right_opaque {
                        return false;
                    }
                }
                Compare::Form(Protoform::Parenthesized(left), Protoform::Parenthesized(right)) => {
                    if left != right {
                        return false;
                    }
                }
                Compare::Form(Protoform::Bare(left), Protoform::Bare(right)) => {
                    if left != right {
                        return false;
                    }
                }
                Compare::Form(
                    Protoform::Qualified(left_symbol, left_forms),
                    Protoform::Qualified(right_symbol, right_forms),
                ) => {
                    if left_symbol != right_symbol || left_forms.len() != right_forms.len() {
                        return false;
                    }
                    self.work.extend(
                        left_forms
                            .iter()
                            .zip(right_forms)
                            .map(|(left, right)| Compare::Form(left, right)),
                    );
                }
                Compare::Form(..) => return false,
                Compare::Head(Head::Symbol(left), Head::Symbol(right)) => {
                    if left != right {
                        return false;
                    }
                }
                Compare::Head(
                    Head::Qualified(left_symbol, left_forms),
                    Head::Qualified(right_symbol, right_forms),
                ) => {
                    if left_symbol != right_symbol || left_forms.len() != right_forms.len() {
                        return false;
                    }
                    self.work.extend(
                        left_forms
                            .iter()
                            .zip(right_forms)
                            .map(|(left, right)| Compare::Form(left, right)),
                    );
                }
                Compare::Head(..) => return false,
            }
        }
        true
    }
    fn heads(left: &'a Head, right: &'a Head) -> bool {
        Self {
            work: vec![Compare::Head(left, right)],
        }
        .same()
    }
    fn forms(left: &'a Protoform, right: &'a Protoform) -> bool {
        Self {
            work: vec![Compare::Form(left, right)],
        }
        .same()
    }
}

impl PartialEq for Head {
    fn eq(&self, other: &Self) -> bool {
        Comparer::heads(self, other)
    }
}
impl Eq for Head {}
impl PartialEq for Protoform {
    fn eq(&self, other: &Self) -> bool {
        Comparer::forms(self, other)
    }
}
impl Eq for Protoform {}

enum SituationCopyJob<'a> {
    Situation(&'a Situation),
    Finish(crate::Extent, usize),
}
struct SituationCopier<'a> {
    work: Vec<SituationCopyJob<'a>>,
    values: Vec<Situation>,
}
trait SituationCopying {
    fn copy(&mut self);
}
impl SituationCopying for SituationCopier<'_> {
    fn copy(&mut self) {
        while let Some(job) = self.work.pop() {
            match job {
                SituationCopyJob::Situation(situation) => {
                    self.work.push(SituationCopyJob::Finish(
                        situation.extent,
                        situation.children.len(),
                    ));
                    self.work.extend(
                        situation
                            .children
                            .iter()
                            .rev()
                            .map(SituationCopyJob::Situation),
                    );
                }
                SituationCopyJob::Finish(extent, count) => {
                    let children = self.values.split_off(self.values.len() - count);
                    self.values.push(Situation { extent, children });
                }
            }
        }
    }
}

struct ComparedSituations<'a> {
    left: &'a Situation,
    right: &'a Situation,
}
struct SituationComparer<'a> {
    work: Vec<ComparedSituations<'a>>,
}
trait SituationComparing {
    fn same(&mut self) -> bool;
}
impl SituationComparing for SituationComparer<'_> {
    fn same(&mut self) -> bool {
        while let Some(ComparedSituations { left, right }) = self.work.pop() {
            if left.extent != right.extent || left.children.len() != right.children.len() {
                return false;
            }
            self.work.extend(
                left.children
                    .iter()
                    .zip(&right.children)
                    .map(|(left, right)| ComparedSituations { left, right }),
            );
        }
        true
    }
}

impl Clone for Situation {
    fn clone(&self) -> Self {
        let mut copier = SituationCopier {
            work: vec![SituationCopyJob::Situation(self)],
            values: Vec::new(),
        };
        copier.copy();
        copier.values.pop().expect("one copied situation")
    }
}
impl PartialEq for Situation {
    fn eq(&self, other: &Self) -> bool {
        let mut comparer = SituationComparer {
            work: vec![ComparedSituations {
                left: self,
                right: other,
            }],
        };
        comparer.same()
    }
}
impl Eq for Situation {}
impl<T: Clone> Clone for Situated<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone(), self.1.clone())
    }
}
impl<T: PartialEq> PartialEq for Situated<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0 && self.1 == other.1
    }
}
impl<T: Eq> Eq for Situated<T> {}
impl Clone for Delineation {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl PartialEq for Delineation {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for Delineation {}

enum Show<'a> {
    Form(&'a Protoform),
    Head(&'a Head),
    Situation(&'a Situation),
    Text(&'static str),
    Symbol(&'a crate::Symbol),
    Bare(&'a crate::Bare),
    Opaque(&'a crate::Opaque),
    TextValue(&'a crate::Text),
    Separator(&'a Separator),
    Enclosure(&'a Enclosure),
    Extent(&'a crate::Extent),
}

struct Displaying<'a> {
    work: Vec<Show<'a>>,
}
trait Showing<'a> {
    fn forms(&mut self, values: &'a [Protoform]);
    fn show(&mut self, f: &mut fmt::Formatter<'_>) -> fmt::Result;
}
impl<'a> Showing<'a> for Displaying<'a> {
    fn forms(&mut self, values: &'a [Protoform]) {
        self.work.push(Show::Text("]"));
        for (index, value) in values.iter().enumerate().rev() {
            self.work.push(Show::Form(value));
            if index != 0 {
                self.work.push(Show::Text(", "));
            }
        }
        self.work.push(Show::Text("["));
    }

    fn show(&mut self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        while let Some(job) = self.work.pop() {
            match job {
                Show::Text(text) => f.write_str(text)?,
                Show::Symbol(value) => write!(f, "{value:?}")?,
                Show::Bare(value) => write!(f, "{value:?}")?,
                Show::Opaque(value) => write!(f, "{value:?}")?,
                Show::Separator(value) => write!(f, "{value:?}")?,
                Show::Enclosure(value) => write!(f, "{value:?}")?,
                Show::TextValue(value) => write!(f, "{value:?}")?,
                Show::Extent(value) => write!(f, "{value:?}")?,
                Show::Form(Protoform::Headed(head, separator, body)) => {
                    self.work.extend([
                        Show::Text(")"),
                        Show::Form(body),
                        Show::Text(", "),
                        Show::Separator(separator),
                        Show::Text(", "),
                        Show::Head(head),
                        Show::Text("Headed("),
                    ]);
                }
                Show::Form(Protoform::Enclosed(enclosure, children)) => {
                    self.work.push(Show::Text(")"));
                    self.forms(children);
                    self.work.push(Show::Text(", "));
                    self.work.push(Show::Enclosure(enclosure));
                    self.work.push(Show::Text("Enclosed("));
                }
                Show::Form(Protoform::Quoted(text)) => self.work.extend([
                    Show::Text(")"),
                    Show::TextValue(text),
                    Show::Text("Quoted("),
                ]),
                Show::Form(Protoform::Parenthesized(opaque)) => self.work.extend([
                    Show::Text(")"),
                    Show::Opaque(opaque),
                    Show::Text("Parenthesized("),
                ]),
                Show::Form(Protoform::Bare(bare)) => {
                    self.work
                        .extend([Show::Text(")"), Show::Bare(bare), Show::Text("Bare(")])
                }
                Show::Form(Protoform::Qualified(symbol, forms)) => {
                    self.work.push(Show::Text(")"));
                    self.forms(forms);
                    self.work.push(Show::Text(", "));
                    self.work.push(Show::Symbol(symbol));
                    self.work.push(Show::Text("Qualified("));
                }
                Show::Head(Head::Symbol(symbol)) => {
                    self.work
                        .extend([Show::Text(")"), Show::Symbol(symbol), Show::Text("Symbol(")])
                }
                Show::Head(Head::Qualified(symbol, constraints)) => {
                    self.work.push(Show::Text(")"));
                    self.forms(constraints);
                    self.work.push(Show::Text(", "));
                    self.work.push(Show::Symbol(symbol));
                    self.work.push(Show::Text("Qualified("));
                }
                Show::Situation(situation) => {
                    self.work.push(Show::Text(" }"));
                    self.work.push(Show::Text("]"));
                    for (index, child) in situation.children.iter().enumerate().rev() {
                        self.work.push(Show::Situation(child));
                        if index != 0 {
                            self.work.push(Show::Text(", "));
                        }
                    }
                    self.work.push(Show::Text("["));
                    self.work.push(Show::Text(", children: "));
                    self.work.push(Show::Extent(&situation.extent));
                    self.work.push(Show::Text("Situation { extent: "));
                }
            }
        }
        Ok(())
    }
}

impl fmt::Debug for Head {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut displaying = Displaying {
            work: vec![Show::Head(self)],
        };
        displaying.show(f)
    }
}
impl fmt::Debug for Protoform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut displaying = Displaying {
            work: vec![Show::Form(self)],
        };
        displaying.show(f)
    }
}
impl fmt::Debug for Situation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut displaying = Displaying {
            work: vec![Show::Situation(self)],
        };
        displaying.show(f)
    }
}
impl<T: fmt::Debug> fmt::Debug for Situated<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Situated")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}
impl fmt::Debug for Delineation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Delineation").field(&self.0).finish()
    }
}
