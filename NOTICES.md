# Third-party notices

Crunchnarr is licensed under [MIT](LICENSE). The published Docker image
(`ghcr.io/stallerr/crunchnarr`) bundles third-party tools that have their
own licenses. Those tools are invoked as separate processes via `fork`/`exec`
and command-line arguments — under the FSF's reading of GPL v2, this is
"mere aggregation" and does not make the rest of Crunchnarr a derivative
work. The MIT license on Crunchnarr's own source code is unaffected.

This file lists the bundled tools and their licenses.

---

## mp4decrypt (Bento4)

- **What it is:** Command-line utility from the Bento4 SDK, used to decrypt
  CENC-encrypted MP4 segments after the Widevine CDM has issued the keys.
- **License:** Dual-licensed [GPL v2](https://www.gnu.org/licenses/old-licenses/gpl-2.0.txt)
  or commercial license from Axiomatic Systems LLC. See <https://www.bento4.com/about/>.
- **How it's bundled:** The `Dockerfile` clones <https://github.com/axiomatic-systems/Bento4>
  at image-build time, compiles `mp4decrypt`, and copies the resulting binary
  into `/usr/local/bin/mp4decrypt` in the final image. The Bento4 source is
  not vendored into Crunchnarr's repository.
- **Source code (per GPL §3):** Available at <https://github.com/axiomatic-systems/Bento4>.
  The exact revision used in any given image is whatever `master` points to
  at the time the image was built.
- **How Crunchnarr uses it:** `crunchy-cli/src/media/mp4decrypt.rs` shells out
  to the binary via `std::process::Command`, passing the encrypted file path,
  output path, and CENC keys as command-line arguments. Crunchnarr does **not**
  link Bento4 as a library, statically or dynamically.

## FFmpeg

- **What it is:** Multimedia framework, used here to remux decrypted segments
  into the final container (MP4 / MKV) and to merge audio/subtitle tracks.
- **License:** [LGPL v2.1](https://www.gnu.org/licenses/old-licenses/lgpl-2.1.txt)+
  for the core; some optional components are GPL. The Debian package shipped
  in our image is built per Debian's distribution policy.
- **How it's bundled:** Installed via `apt-get install -y ffmpeg` in the
  `Dockerfile`. Source is available through Debian's standard apt-source
  channels and from <https://ffmpeg.org/>.
- **How Crunchnarr uses it:** Shelled out to via `std::process::Command` (no
  linking against `libav*`).

## Bento4 — runtime detection (no bundling)

If you build Crunchnarr from source without using the Dockerfile, the project
expects `mp4decrypt` to be on `$PATH` (or set `tools.mp4decrypt` to a custom
path in your config). Crunchnarr does not vendor the Bento4 source tree and
does not ship the binary outside of the Docker image.

---

## Crunchnarr's own license

The Rust crates in `crunchy-cli/`, the Next.js app in `crunchy-web/`, and
everything else under this repository's MIT license header is MIT licensed
and may be used, modified, and redistributed under those terms.

The use of the Widevine CDM at runtime is governed by the agreement between
you and Google. Crunchnarr does not ship a Widevine CDM and does not include
any DRM-bypass code.
