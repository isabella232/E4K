Building this crate will generate the rust code for the workload API server and client.
1. In build.rs, make sure that the build is targetting the lastest stable version of SPIFFE workload API.
2. Build
3. Extract the generated file from target/debug/build/workload-api-xxxxx/_.rs and replace the lib.rs file of the crate with it.
