# Store: the contract between Iris, the server and the web

2026-10-11. The plan is `docs/store-phase-2-plan.md`; this is what every side implements, so
they can't drift apart. Recipes and channels: `docs/channels.md`. Parts in WebAssembly:
`crates/iris-piece` (ABI 2). Shape pieces: `crates/iris-shape`.

## Packages

A package is a zip (`.iris`, stored or deflated) with `iris.toml` at its root and the files its
kind needs. Only these files are allowed; no directories other than the ones listed; no `..`;
5 MB in total, 2 MB per file.

| kind     | format  | files                                   |
|----------|---------|-----------------------------------------|
| `piece`  | `wasm`  | `iris.toml`, `piece.wasm`               |
| `piece`  | `shape` | `iris.toml`, `shape.toml`               |
| `buddy`  | —       | `iris.toml`, `recipe.toml`              |

```toml
[package]
name = "kirby"              # ^[a-z0-9][a-z0-9-]{1,39}$, unique in the store, owned by whoever published it first
kind = "buddy"              # buddy | piece
format = "shape"            # pieces only: wasm | shape
version = "1.0.0"           # x.y.z, higher than the last one published
description = "Kirby, round and pink"
license = "MIT"
```

The author is not in the manifest: the server takes it from the account that uploads.

A recipe (`recipe.toml`, see `src/core/recipe/original.toml`) refers to parts with `use`:

| `use`                 | means                                                         |
|-----------------------|---------------------------------------------------------------|
| `builtin:<name>`      | a built-in part (`builtin:eye`, `builtin:arcs`…)               |
| `<name>@<major>`      | a store piece: the newest installed version with that major   |
| `file:<path>`         | a local `.wasm` part (development only; never in a package)   |
| `shape:<path>`        | a local shape piece (development only; never in a package)    |

A buddy's **dependencies** are its `<name>@<major>` uses; the server computes them and puts them
in the index.

## The index

What Iris trusts. JSON, served at `GET /api/index.json`, signed with Ed25519: the signature is
the base64 of the signature over **the exact bytes** of `index.json`, served at
`GET /api/index.json.sig`. Iris carries the public key in its binary.

```json
{
  "serial": 7,
  "generated_at": "2026-10-11T08:00:00Z",
  "packages": [
    {
      "name": "kirby", "kind": "buddy", "format": null, "version": "1.0.0",
      "description": "Kirby, round and pink", "author": "kexnarv", "license": "MIT",
      "sha256": "…64 hex…", "bytes": 1234, "deps": ["kirby-body@1"],
      "published_at": "2026-10-11T08:00:00Z"
    }
  ],
  "revoked": ["…sha256…"]
}
```

`packages` holds every signed version (not only the latest); Iris picks the newest per name and
major. A version whose `sha256` is in `revoked` is never installed, and if it's installed Iris
turns it off and says why. `serial` only goes up: Iris ignores an index with a lower serial than
the one it already has.

The package itself: `GET /api/pieces/{name}/{version}.iris`. Iris checks its `sha256` against
the signed index before opening it. It adds `?install=1` only when installing the package itself
(not its dependencies, not to look at it), and asks for it even if it already has it from a
preview: that's the only fetch counted as a download, once per IP, version and day.

## Catalog, likes and what's new

| Method and route                 | Who     | Answer                                                                  |
|----------------------------------|---------|--------------------------------------------------------------------------|
| `GET /api/pieces?kind=&q=`       | anyone  | → `[{name, kind, format, version, description, author, license, sha256, bytes, deps, downloads, downloads_week, likes, liked, versions, first_published_at, published_at}]`: the newest signed version of each package. `liked` needs the session (false without it) |
| `GET /api/pieces/{name}`         | anyone  | the same, plus `history: [{version, description, downloads, published_at}]` (newest first) and `daily: [{day, count}]` (installs, last 90 days, with the empty days) |
| `PUT /api/pieces/{name}/like`    | author  | → `{likes, liked: true}`; `DELETE` takes it back → `{likes, liked: false}` |
| `GET /api/releases?limit=`       | anyone  | → newest first, up to 100: `{type: "package", name, kind, format, version, author, description, first, published_at}` (`first`: a new package, not an update) and `{type: "iris", version, notes, channel, published_at}` |

## Banners

The background of a package's page on the web. An author keeps a library of banners, each one
reviewed on its own (nobody else sees it until it's approved); each package points to one banner
or none, and several packages can share one. A banner never changes its image: a new image is a
new banner, back in review. The server redraws every upload (cropped to 1920×800 like CSS
`object-position` at `x`, `y`; saved as JPEG, without metadata). Iris doesn't use them.

| Method and route                              | Who    | Body → answer                                                     |
|-----------------------------------------------|--------|-------------------------------------------------------------------|
| `POST /api/me/banners?x=&y=&label=`           | author | the image (PNG, JPEG or WebP, ≤ 12 MB, ≥ 960×400) → `{id, label, sha256, bytes, status, reason, created_at, used_by}` |
| `GET /api/me/banners`                         | author | → the same, newest first                                          |
| `GET /api/me/banners/{id}/image`              | author | the JPEG, whatever its status                                     |
| `DELETE /api/me/banners/{id}`                 | author | its packages go back to the default background                    |
| `GET /api/me/packages`                        | author | → `[{name, kind, banner_id}]`                                     |
| `PUT /api/me/packages/{name}/banner`          | author | `{banner: id \| null}`; one in review can be chosen (shows once approved), a rejected one can't |
| `GET /api/banners/{sha256}.jpg`               | anyone | approved ones only; immutable                                     |
| `GET /api/admin/banners?status=`              | review.view   | the queue: `[{id, author, label, sha256, status, reason, created_at, used_by}]` |
| `GET /api/admin/banners/{id}/image`           | review.view   | the JPEG, whatever its status                              |
| `POST /api/admin/banners/{id}/approve` · `/reject` | review.decide | `/reject` takes `{reason}`; rejecting takes it off every package |

The catalog (`GET /api/pieces`, `GET /api/pieces/{name}`) carries `banner`: the sha256 of the
package's approved banner, or null.

## Accounts and sessions

Every authenticated call carries `Authorization: Bearer <token>`. Tokens last 90 days; the server
stores only their sha256.

| Method and route                       | Body → answer                                                            |
|----------------------------------------|---------------------------------------------------------------------------|
| `POST /api/auth/register`              | `{username, email, password}` → `{token, username}`. Username like a package name; password ≥ 8 |
| `POST /api/auth/login`                 | `{login, password}` (username or email) → `{token, username}`             |
| `POST /api/auth/github/start`          | → `{device_code, user_code, verification_uri, interval}`; 503 if GitHub isn't configured |
| `POST /api/auth/github/poll`           | `{device_code}` → `{token, username}`, or 202 `{pending: true}` while the user hasn't confirmed |
| `GET /api/me`                          | → `{username, email, github}`                                              |
| `POST /api/auth/logout`                | revokes the token                                                         |
| `GET /api/store/config`                | → `{github: bool}`: whether the GitHub login is configured                |

## Publishing

| Method and route                          | Who    | Body → answer                                                           |
|-------------------------------------------|--------|--------------------------------------------------------------------------|
| `POST /api/pieces`                        | author | the `.iris` bytes (`application/zip`) → `{name, version, status}` or 400 `{error}` in plain Spanish |
| `GET /api/me/pieces`                      | author | → `[{name, kind, version, status, reason, uploaded_at}]`                 |
| `PUT /api/me/drafts/{name}`               | author | `{files: {"recipe.toml": "…", "ears.toml": "…"}}` (text files only) → `{updated_at}` |
| `GET /api/me/drafts`                      | author | → `[{name, updated_at}]`                                                 |
| `GET /api/me/drafts/{name}`               | author | → `{name, updated_at, files}`                                            |
| `DELETE /api/me/drafts/{name}`            | author |                                                                          |
| `POST /api/pieces/{name}/report`          | anyone | `{reason}` → 204                                                         |

`status` is one of `in_review`, `approved`, `signed`, `rejected`, `revoked`.

A draft is a buddy being built in Iris Studio: its `recipe.toml` and its shape pieces, by file
name. Iris, logged in with the same account, shows drafts in `/buddy` and keeps them up to date.

**Validation on upload** (before anything reaches review), with the reason in the error:
1. A sane zip: allowed files only (table above), no `..`, within the limits.
2. The manifest: every field, a known kind and format, a free name or one the uploader owns,
   a version higher than the last one of that name.
3. `wasm`: it parses; imports nothing but `iris.rand`; exports `iris_abi` (returning 2),
   `iris_decl`, `iris_io`, `iris_new`; its declaration parses; it runs `iris_new`, then 300
   frames of `iris_step`, `iris_layout` and `iris_paint` and every event, within the fuel and
   memory limits of Iris's sandbox, without trapping.
4. `shape`: `iris_shape::Shape::parse` accepts it and every formula compiles.
5. `buddy`: the recipe parses; every `use` is `builtin:<known>` or `<name>@<major>` of a signed
   store piece.

## Review (Kevin)

The admin routes take `Authorization: Bearer <admin token>` (the one in `/etc/iris-server.env`).

| Method and route                                   | Body → answer                                                   |
|----------------------------------------------------|------------------------------------------------------------------|
| `GET /api/admin/pieces?status=in_review`           | → `[{name, kind, format, version, author, description, sha256, uploaded_at, status, reason}]` |
| `GET /api/admin/pieces/{name}/{version}.iris`      | the package, whatever its status                                 |
| `POST /api/admin/pieces/{name}/{version}/approve`  | → 204 (status `approved`: not served until the next signed index) |
| `POST /api/admin/pieces/{name}/{version}/reject`   | `{reason}` → 204                                                 |
| `POST /api/admin/pieces/{name}/{version}/revoke`   | `{reason}` → 204 (goes into `revoked` at the next signed index)  |
| `GET /api/admin/index/draft`                       | → the next `index.json` as text: every `signed` and `approved` version, minus revoked, `serial` + 1 |
| `POST /api/admin/index`                            | `{index, signature}` (the exact text from the draft and its base64 signature) → 204. The server checks the signature with the public key it knows (`IRIS_STORE_PUBKEY`), publishes it, and marks the approved versions it contains as `signed` |
| `GET /api/admin/reports`                           | → `[{name, reason, created_at}]`                                 |

The signing key is Ed25519. Its private half lives in Kevin's browser (imported once from
`~/.config/iris/store-signing-key`, PKCS#8 PEM) and never reaches the server; its public half is
in Iris's binary and in the server's environment.

## In Iris

Commands: `iris login` / `iris logout`; `iris piece new <name> [--shape|--wasm|--buddy]`,
`try`, `pack`, `publish`, `status`; in the chat `/store [search]`, `/store update`,
`/store remove <name>`, `/buddy <name>`, `/buddy edit` (opens it in Iris Studio as a draft),
`/buddy publish`. A project's `iris.toml` may carry `[preview] replaces = "body"` so `try` puts
the piece in place of the blob.

Installed packages: `~/.local/share/iris/store/<name>/<version>/` (the unzipped files) on
Linux, `%LOCALAPPDATA%\Iris\store\…` on Windows. The last good index and its signature are kept
next to them (`index.json`, `index.json.sig`). Recipes the user saves: `~/.config/iris/buddies/
<name>/recipe.toml` plus their shape files. The chosen buddy is still `~/.config/iris/buddy`
(now any buddy's name).
