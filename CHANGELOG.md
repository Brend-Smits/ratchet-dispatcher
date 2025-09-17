# Changelog

## [1.6.0](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.5.0...v1.6.0) (2025-09-17)


### Features

* enhance clone_repository with shallow clone and filtering options ([1f17792](https://github.com/Brend-Smits/ratchet-dispatcher/commit/1f17792e9603fd1231061905a7e6aa5bb500bb28))

## [1.5.0](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.4.0...v1.5.0) (2025-09-17)


### Miscellaneous Chores

* release v1.5.0 ([9d6dbf8](https://github.com/Brend-Smits/ratchet-dispatcher/commit/9d6dbf89102027f5aaeb5b1bf8107566a111a30b))

## [1.4.0](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.3.0...v1.4.0) (2025-09-12)


### Features

* add clean_comment option ([ea1cdee](https://github.com/Brend-Smits/ratchet-dispatcher/commit/ea1cdee17a85baf71c668dec4fb0d37a65fc7386))


### Bug Fixes

* improve error handling in workflow upgrades and enhance comment cleaning functionality ([2a5ba83](https://github.com/Brend-Smits/ratchet-dispatcher/commit/2a5ba835eaf212e8ef6d0f34b1dda0e94e559bee))

## [1.3.0](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.2.5...v1.3.0) (2025-09-12)


### Features

* enhance Git and GitHub integration with improved logging and dry-run functionality ([244ebf4](https://github.com/Brend-Smits/ratchet-dispatcher/commit/244ebf416a2fc0745e1b0871e9f82c07f4f0e455))


### Bug Fixes

* add debug logging for modified files in GitRepository and info logging for repository processing ([b0539b1](https://github.com/Brend-Smits/ratchet-dispatcher/commit/b0539b12f94e4f8d6960377e5d1e979814680b98))
* correct artifact name for SBOM action in release workflow ([1e8ad4f](https://github.com/Brend-Smits/ratchet-dispatcher/commit/1e8ad4fe1379201b3de0530a36bc5b4200c79447))
* update Rust container version to 1.85 in CI workflow ([43f303b](https://github.com/Brend-Smits/ratchet-dispatcher/commit/43f303b07c103db92f785a1e20a5481429225c2e))

## [1.2.5](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.2.4...v1.2.5) (2025-09-12)


### Bug Fixes

* update Dockerfile and release workflow for improved compatibility ([ed3a2a5](https://github.com/Brend-Smits/ratchet-dispatcher/commit/ed3a2a56dc211cffff5baee52f72314dd51b20c1))

## [1.2.4](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.2.3...v1.2.4) (2025-09-12)


### Bug Fixes

* simplify error handling and improve result types across multiple modules ([beb9aba](https://github.com/Brend-Smits/ratchet-dispatcher/commit/beb9aba5851b489752c4985535498ae569744970))

## [1.2.3](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.2.2...v1.2.3) (2024-06-29)


### Bug Fixes

* continue if there are errors ([9518596](https://github.com/Brend-Smits/ratchet-dispatcher/commit/9518596d5f4e48528d97148d97c98f5046429454))

## [1.2.2](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.2.1...v1.2.2) (2024-06-14)


### Bug Fixes

* broken authorization when cloning repository ([1a1de17](https://github.com/Brend-Smits/ratchet-dispatcher/commit/1a1de17d84539f42a5d4d1ceb45563a544764e29))

## [1.2.1](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.2.0...v1.2.1) (2024-06-14)


### Bug Fixes

* release assets did not get added properly in some tags ([397c6d7](https://github.com/Brend-Smits/ratchet-dispatcher/commit/397c6d74a7050e20529abac145282a1a1486ca4e))

## [1.2.0](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.1.0...v1.2.0) (2024-06-14)


### Features

* build ratchet from source so it's included in the docker image ([92ca664](https://github.com/Brend-Smits/ratchet-dispatcher/commit/92ca664b02324e71684779715a9f1159b3e106ab))

## [1.1.0](https://github.com/Brend-Smits/ratchet-dispatcher/compare/v1.0.0...v1.1.0) (2024-06-05)


### Features

* add initial working version ([aabdaf2](https://github.com/Brend-Smits/ratchet-dispatcher/commit/aabdaf20b0e1be4095ae51d3861ace50e89ecde4))
* add possibility to set clone directory and add default branch value ([113fa3e](https://github.com/Brend-Smits/ratchet-dispatcher/commit/113fa3e88f405766bca6aeb16809e178d90df4a2))
* add support for multiple repositories ([4735050](https://github.com/Brend-Smits/ratchet-dispatcher/commit/47350503290bd5efbbf59c9614a7d9b1c9cd4a2c))
* overwrite pull request if it already exists (force push) ([5b536e5](https://github.com/Brend-Smits/ratchet-dispatcher/commit/5b536e553e17c79ed4a6c699507abdb3795ced1d))
* revert ratchet removing newlines or blank lines ([f56f27e](https://github.com/Brend-Smits/ratchet-dispatcher/commit/f56f27e1c1e1e609a0e94da0896f995f0d800e87))
