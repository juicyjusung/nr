# Changelog

## [0.2.0](https://github.com/juicyjusung/nr/compare/v0.1.0...v0.2.0) (2026-02-09)


### Features

* add environment variables and script arguments configuration ([59159c2](https://github.com/juicyjusung/nr/commit/59159c28e697a6aa0887845a1e9e9b37053683af))
* **errors:** improve error messages with actionable guidance ([60bd949](https://github.com/juicyjusung/nr/commit/60bd949ad7973ff961e58a5319db50d0e13f328a))
* **store:** project-scoped storage with per-project isolation and CLI reset ([e6493a6](https://github.com/juicyjusung/nr/commit/e6493a660c5b59638a7ce08532a12d3ad238e6e6))


### Bug Fixes

* **ci:** checkout main branch in scoop job to avoid detached HEAD ([51638dc](https://github.com/juicyjusung/nr/commit/51638dc8fef8bf99aac97a71ebf3b497f8a7a4c4))
* clippy warning - derive Default for ArgsHistory ([ff7b005](https://github.com/juicyjusung/nr/commit/ff7b005ccb81ec36b68a3518a0c98eec74015a07))

## 0.1.0 (2026-02-08)


### Features

* **ui:** add header bar with project name, path, and package manager ([bc19397](https://github.com/juicyjusung/nr/commit/bc193970f3cae34e92ff34ee4e793fdee2c4444c))


### Bug Fixes

* normalize path separators in workspace relative paths for Windows ([476d1fa](https://github.com/juicyjusung/nr/commit/476d1fae48d4ada93c91396c70587ba0a8315a44))
