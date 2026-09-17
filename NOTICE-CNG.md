# RustDesk CNG — modified distribution

Modification date: 2026-09-16.

This is a modified version of RustDesk, not an official upstream release.
Original copyright and attribution notices, including those of Purslane Ltd.
and other contributors, are retained in the source files and dependencies.
This modified work is distributed under GNU Affero General Public License
version 3; see LICENCE. You may redistribute and modify it under that license.
It is provided WITHOUT ANY WARRANTY, including merchantability or fitness
for a particular purpose, except where separately agreed in writing.
Dependency licenses and notices remain applicable to their respective code.

The client and its local hbb_common are based on rustdesk/rustdesk commit
4b066b1fbaa8d5d6f9b53cb1e5b25e484f229da1 (client version 1.3.6).
Changes include Windows CNG certificate authentication, a configurable
compiled-in server address and public key, layered transport/session
encryption, local historical hbb_common sources, certificate error messages,
and the GDI capture fallback. See MIGRATION-1.3.6.md for details.

The corresponding source for each binary release must be offered beside
the application download, through the distributor's source repository or
release archive. A link to an unrelated upstream branch is insufficient.
No production private keys, technician private keys or user database are
required in that public source distribution.
