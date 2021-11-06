(function() {var implementors = {};
implementors["festival_api"] = [{"text":"impl&lt;__DB:&nbsp;Backend&gt; QueryableByName&lt;__DB&gt; for <a class=\"struct\" href=\"festival_api/models/struct.User.html\" title=\"struct festival_api::models::User\">User</a> <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.56.1/std/primitive.i32.html\">i32</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/users/columns/struct.id.html\" title=\"struct festival_api::schema::users::columns::id\">id</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.56.1/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/users/columns/struct.usr.html\" title=\"struct festival_api::schema::users::columns::usr\">usr</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.56.1/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/users/columns/struct.pwd.html\" title=\"struct festival_api::schema::users::columns::pwd\">pwd</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/datetime/struct.DateTime.html\" title=\"struct chrono::datetime::DateTime\">DateTime</a>&lt;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/offset/utc/struct.Utc.html\" title=\"struct chrono::offset::utc::Utc\">Utc</a>&gt;: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/users/columns/struct.lckdwn.html\" title=\"struct festival_api::schema::users::columns::lckdwn\">lckdwn</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/datetime/struct.DateTime.html\" title=\"struct chrono::datetime::DateTime\">DateTime</a>&lt;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/offset/utc/struct.Utc.html\" title=\"struct chrono::offset::utc::Utc\">Utc</a>&gt;: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/users/columns/struct.crt.html\" title=\"struct festival_api::schema::users::columns::crt\">crt</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/datetime/struct.DateTime.html\" title=\"struct chrono::datetime::DateTime\">DateTime</a>&lt;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/offset/utc/struct.Utc.html\" title=\"struct chrono::offset::utc::Utc\">Utc</a>&gt;: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/users/columns/struct.last_accessed.html\" title=\"struct festival_api::schema::users::columns::last_accessed\">last_accessed</a>&gt;, __DB&gt;,&nbsp;</span>","synthetic":false,"types":["festival_api::models::User"]},{"text":"impl&lt;__DB:&nbsp;Backend&gt; QueryableByName&lt;__DB&gt; for <a class=\"struct\" href=\"festival_api/models/struct.GenerationRequest.html\" title=\"struct festival_api::models::GenerationRequest\">GenerationRequest</a> <span class=\"where fmt-newline\">where<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.56.1/std/primitive.i32.html\">i32</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/reqs/columns/struct.id.html\" title=\"struct festival_api::schema::reqs::columns::id\">id</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.56.1/std/primitive.i32.html\">i32</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/reqs/columns/struct.usr_id.html\" title=\"struct festival_api::schema::reqs::columns::usr_id\">usr_id</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/datetime/struct.DateTime.html\" title=\"struct chrono::datetime::DateTime\">DateTime</a>&lt;<a class=\"struct\" href=\"https://docs.rs/chrono/latest/chrono/offset/utc/struct.Utc.html\" title=\"struct chrono::offset::utc::Utc\">Utc</a>&gt;: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/reqs/columns/struct.crt.html\" title=\"struct festival_api::schema::reqs::columns::crt\">crt</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.56.1/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/reqs/columns/struct.word.html\" title=\"struct festival_api::schema::reqs::columns::word\">word</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"struct\" href=\"https://doc.rust-lang.org/1.56.1/alloc/string/struct.String.html\" title=\"struct alloc::string::String\">String</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/reqs/columns/struct.lang.html\" title=\"struct festival_api::schema::reqs::columns::lang\">lang</a>&gt;, __DB&gt;,<br>&nbsp;&nbsp;&nbsp;&nbsp;<a class=\"primitive\" href=\"https://doc.rust-lang.org/1.56.1/std/primitive.f32.html\">f32</a>: FromSql&lt;SqlTypeOf&lt;<a class=\"struct\" href=\"festival_api/schema/reqs/columns/struct.speed.html\" title=\"struct festival_api::schema::reqs::columns::speed\">speed</a>&gt;, __DB&gt;,&nbsp;</span>","synthetic":false,"types":["festival_api::models::GenerationRequest"]}];
if (window.register_implementors) {window.register_implementors(implementors);} else {window.pending_implementors = implementors;}})()