# Contributing to Rumpel

## Reporting issues

Search existing issues before opening a new one. A useful bug report includes
the Rumpel version, distribution, desktop session, GStreamer version, steps to
reproduce the issue, expected behaviour, and actual behaviour.

Do not report security vulnerabilities in public issues. Follow
[SECURITY.md](SECURITY.md) instead.

## Pull requests

Keep each pull request focused. Describe the user-visible change, include tests
when behaviour changes, and update documentation or the changelog where needed.
Before submitting, run:

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
```

## Licensing contributions

By submitting a contribution, you certify that you have the right to submit it
under the GNU General Public License version 3 or any later version, and agree
that the project may distribute it under those terms.