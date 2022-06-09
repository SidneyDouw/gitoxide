use bstr::BStr;

/// Usually the first methods to call when parsing a rev-spec to set an anchoring revision (which is typically a `Commit` object).
/// Methods can be called multiple time to either try input or to parse another rev-spec that is part of a range.
///
/// In one case they will not be called at all, e.g. `@{[-]n}` indicates the current branch (what `HEAD` dereferences to),
/// without ever naming it, and so does `@{upstream}` or `@{<date>}`.
///
/// Note that when dereferencing `HEAD` implicitly, a revision must be set for later navigation.
pub trait Revision {
    /// Resolve `name` as reference which might not be a valid reference name. The name may be partial like `main` or full like
    /// `refs/heads/main` solely depending on the users input.
    /// Symbolic referenced should be followed till their object, but objects **must not yet** be peeled.
    fn find_ref(&mut self, name: &BStr) -> Option<()>;

    /// An object prefix to disambiguate, returning `None` if it is ambiguous or wasn't found at all.
    fn disambiguate_prefix(&mut self, prefix: git_hash::Prefix) -> Option<()>;

    /// Lookup the reflog of the previously set reference, or dereference `HEAD` to its reference
    /// to obtain the ref name (as opposed to `HEAD` itself).
    /// If there is no such reflog entry, return `None`.
    fn reflog(&mut self, query: ReflogLookup) -> Option<()>;

    /// When looking at `HEAD`, `branch_no` is the non-null checkout in the path, e.g. `1` means the last branch checked out,
    /// `2` is the one before that.
    /// Return `None` if there is no branch as the checkout history (via the reflog) isn't long enough.
    fn nth_checked_out_branch(&mut self, branch_no: usize) -> Option<()>;

    /// Lookup the previously set branch or dereference `HEAD` to its reference to use its name to lookup the sibling branch of `kind`
    /// in the configuration (typically in `refs/remotes/…`). The sibling branches are always local tracking branches.
    /// Return `None` of no such configuration exists and no sibling could be found, which is also the case for all reference outside
    /// of `refs/heads/`.
    /// Note that the caller isn't aware if the previously set reference is a branch or not and might call this method even though no reference
    /// is known.
    fn sibling_branch(&mut self, kind: SiblingBranch) -> Option<()>;
}

/// Combine one or more specs into a range of multiple.
pub trait Kind {
    /// Set the kind of the spec, which happens only once if it happens at all.
    /// In case this method isn't called, assume `Single`.
    /// Reject a kind by returning `None` to stop the parsing.
    ///
    /// Note that ranges don't necessarily assure that a second specification will be parsed.
    /// If `^rev` is given, this method is called with [`spec::Kind::Range`][crate::spec::Kind::Range]
    /// and no second specification is provided.
    fn kind(&mut self, kind: crate::spec::Kind) -> Option<()>;
}

/// Once an anchor is set one can adjust it using traversal methods.
pub trait Navigate {
    /// Adjust the current revision to traverse the graph according to `kind`.
    fn traverse(&mut self, kind: Traversal) -> Option<()>;

    /// Peel the current object until it reached `kind` or `None` if the chain does not contain such object.
    fn peel_until(&mut self, kind: PeelTo<'_>) -> Option<()>;

    /// Find the first revision/commit whose message matches the given `regex` (which is never empty).
    /// to see how it should be matched.
    /// If `negated` is `true`, the first non-match will be a match.
    ///
    /// If no revision is known yet, find the _youngest_ matching commit from _any_ reference, including `HEAD`.
    /// Otherwise, only find commits reachable from the set revision.
    fn find(&mut self, regex: &BStr, negated: bool) -> Option<()>;
}

/// A lookup into the reflog of a reference.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum ReflogLookup {
    /// Lookup by entry, where `0` is the most recent entry, and `1` is the older one behind `0`.
    Entry(usize),
    Date(git_date::Time),
}

/// Define how to traverse the commit graph.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum Traversal {
    /// Select the given parent commit of the currently selected commit, start at `1` for the first parent.
    /// The value will never be `0`.
    NthParent(usize),
    /// Select the given ancestor of the currently selected commit, start at `1` for the first ancestor.
    /// The value will never be `0`.
    NthAncestor(usize),
}

/// Define where a tag object should be peeled to.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum PeelTo<'a> {
    /// An object of the given kind.
    ObjectKind(git_object::Kind),
    /// Ensure the object at hand exists, without imposing any restrictions to its type.
    ExistingObject,
    /// Follow an annotated tag object recursively until an object is found.
    RecursiveTagObject,
    /// The path to drill into as seen relative to the current tree-ish.
    ///
    /// Note that the path can be relative, and `./` and `../` prefixes are seen as relative to the current
    /// working directory.
    ///
    /// The path may be empty, which makes it refer to the tree at the current revision, similar to `^{tree}`.
    /// Note that paths like `../` are valid and refer to a tree as seen relative to the current working directory.
    Path(&'a BStr),
}

/// The kind of sibling branch to obtain.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub enum SiblingBranch {
    /// The upstream branch as configured in `branch.<name>.remote` or `branch.<name>.merge`.
    Upstream,
    /// The upstream branch to which we would push.
    Push,
}

impl SiblingBranch {
    /// Parse `input` as branch representation, if possible.
    pub fn parse(input: &BStr) -> Option<Self> {
        if input.eq_ignore_ascii_case(b"u") || input.eq_ignore_ascii_case(b"upstream") {
            SiblingBranch::Upstream.into()
        } else if input.eq_ignore_ascii_case(b"push") {
            SiblingBranch::Push.into()
        } else {
            None
        }
    }
}
