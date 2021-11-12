initSidebarItems({"enum":[["SearchItem","A search item to be used for finding various database interactions when the flexibility between searching via a users name, and a users id is required."]],"fn":[["compare_hashed_strings","A function which checks whether the first string can be hashed into the second string. Returns a boolean true if they are the same, and false otherwise. In the event the strings cannot be compared due to an error, returns an Err(response) which may be returned to the user. Example Usage:"],["find_user_in_db","Attempt to find a user in the database, returns None if the user is unable to be found. Note that the provided name is assumed unique. If multiple results exist, the first will be returned. If the database interaction fails, returns a response which can be shown to the user."],["get_time_since","Get the time (in seconds) since a chrono datetime. Returns a duration which can be negative if the time is in the future."],["hash_string_with_salt","Hash a string with a random salt to be stored in the database. Utilizing the argon2id algorithm Followed best practices as laid out here: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html Example Usage"],["is_user_timed_out","Returns Ok(()) if the user is not timed out. If the user is timed out returns Err(Response) with a custom message containing the number of seconds the user has left before becoming non-timed out."],["load_recent_requests","Load a users most recent requests, limited based on the number of requests."],["log_request","Uploads the provided phrase_package as a request to the database. This is important for rate limiting, among other things."],["sha_512_hash","Takes an input reference string, and hashes it using the sha512 algorithm. The resultant value is returned as a string in hexadecmial - meaning it is url and i/o safe. The choice of sha512 over sha256 is that sha512 tends to perform better at  longer strings - which we are likely to encounter with this api. Users the sha2 crate internally for hashing."],["update_user_last_seen","Attempts to find and then update a user with a new timestamp."]]});