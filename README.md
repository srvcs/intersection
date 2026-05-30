# srvcs-intersection

The set-intersection service of the srvcs.cloud distributed standard library.

Its single concern: **which values appear in both sets?** Given two lists of
integers `a` and `b`, it reports the sorted list of **distinct** values that
appear in **both** lists.

`srvcs-intersection` is a **leaf**: it depends on no other service and makes no
network calls. All work is local.

```text
result = sorted list of distinct integers present in both a and b
intersection([1, 2, 3], [2, 3, 4]) = [2, 3]
```

## API

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/` | Service identity, concern, and dependency list |
| `POST` | `/` | Report the intersection of sets `a` and `b` |
| `GET` | `/healthz` `/readyz` `/metrics` `/openapi.json` | srvcs service standard surface |

```sh
curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": [1, 2, 3], "b": [2, 3, 4]}'
# {"a":[1,2,3],"b":[2,3,4],"result":[2,3]}

curl -s -X POST localhost:8080/ -H 'content-type: application/json' -d '{"a": [3, 3, 1, 2], "b": [2, 2, 3, 1]}'
# {"a":[3,3,1,2],"b":[2,2,3,1],"result":[1,2,3]}
```

Responses:

- `200 {"a": [...], "b": [...], "result": [<int>, ...]}` — evaluated. `result`
  is the sorted list of distinct integers appearing in both `a` and `b`.
- `422 {"error": "a and b must be lists of integers"}` — some element of `a` or
  `b` is not a JSON integer.

The result is always a sorted, duplicate-free list of `i64`. Duplicates within a
list collapse, and order in the input is irrelevant. Disjoint or empty inputs
yield `[]`.

## Dependencies

None. `srvcs-intersection` is a leaf set service. Because it owns its own
validation, it rejects any non-integer element directly with `422` rather than
forwarding to a dependency.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `SRVCS_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `SRVCS_ENV` | `development` | Environment label for logs |
| `RUST_LOG` | `info,tower_http=info` | Tracing filter |

## Local checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

See [`srvcs/platform`](https://github.com/srvcs/platform) for the shared
standard.

> Note: the `cargoHash` in `flake.nix` is inherited from the template and must be
> refreshed with a `nix build` before the Nix gates pass.
