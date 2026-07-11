# Changelog

## [0.2.3](https://github.com/juicyjusung/nr/compare/v0.2.2...v0.2.3) (2026-07-11)


### Bug Fixes

* **args:** make editing unicode-safe ([ce24e10](https://github.com/juicyjusung/nr/commit/ce24e1058ce2d7b3ca4c59f3dea98baf95b24ebb))
* harden runtime, persistence, and release contracts ([1d9352b](https://github.com/juicyjusung/nr/commit/1d9352bc83bac0699f0c36d13119ac73b34a1083))
* **runner:** align failure command display ([c755b0d](https://github.com/juicyjusung/nr/commit/c755b0d81b078c3c32791fda174f023ade9ff5d0))
* **runner:** use explicit package manager run ([5c26d0c](https://github.com/juicyjusung/nr/commit/5c26d0c16b1f27eec9e50ea6499d4b95183fe275))
* **sort:** preserve fuzzy relevance ([7996c4d](https://github.com/juicyjusung/nr/commit/7996c4d2d585be72b7c779ff5dc36cc4de052e3a))
* **store:** report persistence failures ([7a71632](https://github.com/juicyjusung/nr/commit/7a71632da123e06289fe915d55f9da35de704f47))
* **tui:** finalize session state reliably ([3a812d5](https://github.com/juicyjusung/nr/commit/3a812d5457661895dae3446aa765b0a8e9172c6c))

## [0.2.2](https://github.com/juicyjusung/nr/compare/v0.2.1...v0.2.2) (2026-02-09)


### Bug Fixes

* make release creation idempotent for release-please compatibility ([674891f](https://github.com/juicyjusung/nr/commit/674891fc9c445b36e98be8aebd3da3fa3fe51bba))

## [0.2.1](https://github.com/juicyjusung/nr/compare/v0.2.0...v0.2.1) (2026-02-09)


### Bug Fixes

* remove 'c' key shortcut for configuration entry ([f633e39](https://github.com/juicyjusung/nr/commit/f633e39340c94cf2627e87a701246776d5273e8f))
* remove 'c' key shortcut for configuration entry ([7fa12b5](https://github.com/juicyjusung/nr/commit/7fa12b5f7c6a311f30be30ac2feec6ddb773297e))

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
