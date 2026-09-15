# Bundled fonts

## Inter.ttf

Licensed **SIL Open Font License 1.1** — see [LICENSE-Inter.txt](LICENSE-Inter.txt).
Upstream: <https://github.com/rsms/inter>

The OFL is not the crate's MIT licence, and the two have different obligations.
The one that matters here: the licence and copyright notice must travel with the
font. `src/util/font.rs` does `include_bytes!` on this file, so every binary
built against crownuikit redistributes it — which is why the licence sits next
to it in the published `.crate` rather than only in a repository someone might
not have.

"Inter" is a Reserved Font Name under the OFL. Redistributing the file unchanged
under that name is fine; a *modified* version may not keep the name.
