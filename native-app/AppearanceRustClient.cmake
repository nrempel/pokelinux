# One shared Rust appearance client engine. Qt keeps transport/presentation.
# All consumers in one CMake tree share exactly one selected engine target.
include_guard(GLOBAL)
set(appearance_client_core_dir "${CMAKE_CURRENT_LIST_DIR}/appearance-client-core")
set(POKELINUX_APPEARANCE_CLIENT_ARCHIVE "" CACHE FILEPATH "Explicit already-built matching appearance client engine archive")
set(POKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256 "" CACHE STRING "Required digest for an explicit appearance client engine archive")
if((POKELINUX_APPEARANCE_CLIENT_ARCHIVE AND NOT POKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256) OR (POKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256 AND NOT POKELINUX_APPEARANCE_CLIENT_ARCHIVE))
 message(FATAL_ERROR "Explicit appearance archive and SHA256 must be supplied together")
endif()
if(POKELINUX_APPEARANCE_CLIENT_ARCHIVE)
 file(SHA256 "${POKELINUX_APPEARANCE_CLIENT_ARCHIVE}" actual_appearance_client_sha256)
 if(NOT actual_appearance_client_sha256 STREQUAL POKELINUX_APPEARANCE_CLIENT_ARCHIVE_SHA256)
  message(FATAL_ERROR "Explicit appearance client archive digest mismatch")
 endif()
 set(appearance_client_archive "${POKELINUX_APPEARANCE_CLIENT_ARCHIVE}")
else()
 if(CMAKE_CROSSCOMPILING)
  message(FATAL_ERROR "Cross builds require an explicit pinned appearance client archive for the matching target")
 endif()
 find_program(CARGO_EXECUTABLE cargo REQUIRED)
 set(appearance_client_archive "${CMAKE_CURRENT_BINARY_DIR}/appearance-rust-target/release/libpokelinux_appearance_client.a")
 # Always invoke Cargo so transitive path dependencies participate in rebuilds.
 add_custom_target(appearance-client-rust-build
  COMMAND ${CMAKE_COMMAND} -E env "CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/appearance-rust-target"
   ${CARGO_EXECUTABLE} build --manifest-path "${appearance_client_core_dir}/Cargo.toml" --locked --offline --release
  BYPRODUCTS "${appearance_client_archive}"
  VERBATIM)
endif()
find_package(Threads REQUIRED)
add_library(appearance-client-core STATIC IMPORTED GLOBAL)
set_target_properties(appearance-client-core PROPERTIES
 IMPORTED_LOCATION "${appearance_client_archive}"
 INTERFACE_INCLUDE_DIRECTORIES "${appearance_client_core_dir}/include")
target_link_libraries(appearance-client-core INTERFACE Threads::Threads ${CMAKE_DL_LIBS} m)
if(TARGET appearance-client-rust-build)
 add_dependencies(appearance-client-core appearance-client-rust-build)
endif()
