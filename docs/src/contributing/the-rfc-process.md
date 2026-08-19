# The RFC process

Design decisions of any size live as RFCs under
[`rfcs/`](https://github.com/apimokka/apimock-rs/tree/main/rfcs) in the
repository, governed by
[RFC 000](https://github.com/apimokka/apimock-rs/blob/main/rfcs/done/000-rfc-lifecycle-policy.md).
This page summarises the shape of that process; RFC 000 is the
authority on it.

## The short version

- `rfcs/proposed/` — written and under review; not yet approved.
- `rfcs/accepted/` — approved by the project owner. Implementation may
  start, or may already be finished and merged, but the work has not
  been released yet.
- `rfcs/done/` — released; the historical record.
- `rfcs/archive/` — withdrawn or superseded.

The `accepted/` step exists because approving a design and shipping it
are separate events here, performed by different people. Without a
folder for the gap between them, approved-but-unreleased RFCs sat in
`proposed/` with a `Status` claiming they still awaited approval — the
exact folder/field disagreement described next.

**The folder is the source of truth for an RFC's state**, not the
`Status` field written inside the file. The field is kept consistent
with the folder as a matter of hygiene — update it in the same commit
that moves the file — but if the two ever disagree, the folder wins.
RFC 000 names this failure mode directly: a `Status: Proposed` file
sitting in `done/` tells a reader two different things at once, and
that's a defect in the document, not a detail to shrug off.

Numbers are assigned once, when an RFC is first created, and are never
reused — a withdrawn RFC's number stays retired in `archive/`
permanently rather than being freed up.

## Where this documentation fits in

Nothing on this page — or anywhere else in this documentation site — is
a substitute for reading the RFC that actually decided something. Where
a page states *why* apimock behaves a particular way, and that reason
traces to a specific RFC, the page says so; the RFC itself is the
detailed record.
