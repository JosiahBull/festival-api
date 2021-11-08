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

## Fedora

To develop this api, you must have [Rust](https://www.rust-lang.org/tools/install) installed.

This api also depends on Postgres, Sox, and Festival, and the festvox_cmu_us_aew lang pack.

### Dependencies

```sh
    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    
    # Install Build Tools
    sudo dnf groupinstall @development-tools @development-libraries

    # Install Docker
    # Follow here: https://docs.docker.com/engine/install/fedora/

    # Install Dependencies
    sudo dnf install festival sox libpq libpq-devel
    cargo install diesel_cli --no-default-features --features postgres
    cargo install rustfmt #for automatic code formatting
    cargo install clippy #collection of useful lints
    cargo install cargo-tarpaulin #test coverage

    #Install default language pack
    wget http://www.festvox.org/packed/festival/2.5/voices/festvox_cmu_us_aew_cg.tar.gz
    tar -xf festvox_cmu_us_aew_cg.tar.gz
    sudo mkdir -p /usr/share/festival/voices/us/
    sudo cp -r festival/lib/voices/us/cmu_us_aew_cg/ /usr/share/festival/voices/us/
    rm -rd ./festival
    rm festvox_cmu_us_aew_cg.tar.gz
```

### Development
```sh
# Spawn a postgres backing db
docker run --name fest-db -p 5432:5432 -e POSTGRES_PASSWORD=postgres -N 500 -d postgres
export DATABASE_URL=postgres://postgres:postgres@localhost/fest_api

# Run migration to configure db ready for api
diesel setup

# Start the api, initial compilation may take some time so get a cup of tea
export JWT_SECRET=<Your_Token_Here>
cargo test -- --test-threads 4
cargo run
```

# Contributing

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