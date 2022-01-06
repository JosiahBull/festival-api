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
    "speed": 0.7,
    "fmt": "wav"
}
```
and returns a file which may be streamed or played for a user.

Detailed documentation on how to use the API can be found [here](https://josiahbull.github.io/festival-api/).

# Deployment
## Docker

```sh
git clone https://github.com/JosiahBull/festival-api
cd ./festival-api
cp .example.env .env
nano .env #Update required configuration options for TLS encryption

nano ./config/general.toml #Update any general configuration options you wish to include.
nano ./config/langs.toml #Update any special languages you wish to include (ensure to modify backend.Dockerfile to install them).

docker volume create api-pgdata

chmod +x ./initalize_server.sh

docker-compose --env-file .env up #This takes a long time
```

# Development

Main TODOs to a 0.3 release:
- [ ] Remove temporary files from converter and generator (add configuration)
- [ ] Update docker container, ci/cd to use ffmpeg instead of sox
- [ ] Create a docker image which anyone can get easily
- [ ] Host the project
- [ ] Move tests to their own integration folder (i.e. ./tests, instead of ./src/tests)
- [ ] Improve test coverage above 90%
- [ ] Write documentation all over
- [ ] Develop Maori Flite voices
- [ ] Reimplement speed selection + add a test for it

## Fedora

To develop this api, you must have [Rust](https://www.rust-lang.org/tools/install) installed.

This api also depends on Postgres, Ffmpeg, and Flite.

### Dependencies

```sh
    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    
    # Install Build Tools
    sudo dnf groupinstall @development-tools @development-libraries

    # Install Docker
    # Follow here: https://docs.docker.com/engine/install/fedora/

    # Install Dependencies
    sudo dnf install festival flite ffmpeg libpq libpq-devel
    cargo install diesel_cli --no-default-features --features postgres
    cargo install rustfmt #for automatic code formatting
    cargo install clippy #collection of useful lints
    cargo install cargo-tarpaulin #test coverage
```

### Development
```sh
# Spawn a postgres backing db
docker run --name fest-db -p 5432:5432 -e POSTGRES_PASSWORD=postgres -d postgres -N 500
export DATABASE_URL=postgres://postgres:postgres@localhost/fest_api

# Run migration to configure db ready for api
diesel setup

# Start the api, initial compilation may take some time so get a cup of tea
export JWT_SECRET=<Your_Token_Here>
cargo test -- --test-threads 4
cargo run
```

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

# Contributing

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Code Guidelines
**Please write tests** if we have good test coverage we can avoid any bugs down the line.


Outside of this we use standard Rust formatting for code. This will be enforced through use of [clippy](https://github.com/rust-lang/rust-clippy) and [rustfmt](https://github.com/rust-lang/rustfmt).

## Commit Guidelines
In all commits, please try to follow the [convention for commits](https://www.conventionalcommits.org/en/v1.0.0/#specification).

Ideally aim to push every commit you make, rather than accumulating a large number of commits before pushing, this helps to keep everyone on the same
codebase when collaborating. 

The exception for this is that you should not commit non-compiling code to the main branch. Open a new branch and 
commit to that instead.

## Use of Pull Requests
Outside of exceptional cases, please always push commits to a new branch and then generate a pull request with your new feature. Automated actions will attempt to validate that your code does not break anything existing, but this doesn't guarantee your code works. Please write tests!