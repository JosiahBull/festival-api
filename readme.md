[![codecov](https://codecov.io/gh/JosiahBull/festival-api/branch/main/graph/badge.svg?token=ISOL8A7QVA)](https://codecov.io/gh/JosiahBull/festival-api)
![Build](https://github.com/JosiahBull/festival-api/actions/workflows/test.yml/badge.svg)
[![Docs](https://github.com/JosiahBull/festival-api/actions/workflows/docs.yml/badge.svg)]()
# Festival to Wav API
A simple REST api which takes a request body of the form:
```json
{
    "word": "university",
    "lang": "en",
    "speed": 0.7
}
```
and returns a `.mp3` file which may be streamed or played for a user.

## Getting Started

Currently this api is not setup with a service such as docker, so you must have [Rust](https://www.rust-lang.org/tools/install) installed to use this api.

This api depends on Rust, Rocket, Diesel and Postgres.

```sh
# Spawn a postgres backing db
docker run --name fest-db -p 5432:5432 -e POSTGRES_PASSWORD=hunter42 -d postgres
echo DATABASE_URL=postgres://postgres:hunter42@localhost/fest_api > .env

# Install the diesel cli utility
cargo install diesel_cli --no-default-features --features postgres

# Run migration to configure db ready for api
diesel migration run

# Start the api, initial compilation may take some time so get a cup of tea
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