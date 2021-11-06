[![codecov](https://codecov.io/gh/JosiahBull/festival-api/branch/main/graph/badge.svg?token=ISOL8A7QVA)](https://codecov.io/gh/JosiahBull/festival-api)
![Build](https://github.com/JosiahBull/festival-api/actions/workflows/test.yml/badge.svg)
[![Docs](https://github.com/JosiahBull/festival-api/actions/workflows/docs.yml/badge.svg)](https://josiahbull.github.io/festival-api/festival_api/index.html)
[![OAS Docs](https://github.com/JosiahBull/festival-api/actions/workflows/redoc.yml/badge.svg)](https://josiahbull.github.io/festival-api/)
# Festival to WAV API
A simple REST api which takes a request body of the form:
```json
{
    "word": "university",
    "lang": "en",
    "speed": 0.7
}
```
and returns a `.wav` file which may be streamed or played for a user.

Detailed documentation on how to use the API can be found [here](https://josiahbull.github.io/festival-api/).

## Progress to 0.1-Beta Release
- [x] Write OAS Spec
- [x] Create db schema
- [x] Setup Gh actions
    - [x] Code Coverage
    - [x] Automatic Formatting + Clippy
    - [x] Running Tests
    - [x] Generation of docs + oas for gh-pages.
- [x] Write Rust/Rocket boilerplate + macros
- [x] Student creation/login endpoints 
    - [x] Login Endpoint
    - [x] Account Creation Endpoint
    - [x] JWT Tokens + Validation
    - [x] Tests 
- [ ] Conversion endpoint
    - [x] Validation of Requests from user
    - [x] Generation of wav files
    - [x] Account Rate Limiting
    - [ ] Conversion from .wav to .mp3 or any other fileformat
    - [ ] Tests
- [ ] Configuration
    - [x] Language Configuration `/config/langs.toml`
    - [ ] File Format Configuration `/config/formats.toml`
    - [x] General Configuration `/config/general.toml`
- [ ] Setup docker-compose
- [ ] Code Comments/Documentation

## Getting Started

Currently this api is not setup with a service such as docker, so you must have [Rust](https://www.rust-lang.org/tools/install) installed.

This api depends on Rust, Rocket, Diesel, Postgres, and Festival.

**Note: Festival may not have the default lang `voice_kal_diphone` installed for your system! To fix this change the english voice in `./config/langs.toml` to match what you wish to use on your system.**

```sh
# Spawn a postgres backing db
docker run --name fest-db -p 5432:5432 -e POSTGRES_PASSWORD=postgres -d postgres
export DATABASE_URL=postgres://postgres:postgres@localhost/fest_api

# Install the diesel cli utility
cargo install diesel_cli --no-default-features --features postgres

# Run migration to configure db ready for api
diesel setup

# Start the api, initial compilation may take some time so get a cup of tea
export JWT_SECRET=<Your_Token_Here>
cargo test -- --test-threads 1
cargo run --release
```

## Contributing

### Code Guidelines
**Please write tests** if we have good test coverage we can avoid any bugs down the line.


Outside of this we use standard Rust formatting for code. This will be enforced through use of [clippy](https://github.com/rust-lang/rust-clippy) and [rustfmt](https://github.com/rust-lang/rustfmt).

### Commit Guidelines
In all commits, please try to follow the [convention for commits](https://www.conventionalcommits.org/en/v1.0.0/#specification).

Ideally aim to push every commit you make, rather than accumulating a large number of commits before pushing, this helps to keep everyone on the same
codebase when collaborating. 

The exception for this is that you should not commit non-compiling code to the main branch. Open a new branch and 
commit to that instead.

## Use of Pull Requests
Outside of exceptional cases, please always push commits to a new branch and then generate a pull request with your new feature. Automated actions will attempt to validate that your code does not break anything existing, but this doesn't guarantee your code works. Please write tests!