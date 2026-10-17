# Shared read-only Rust battery client; Qt only presents copied state.
include_guard(GLOBAL)
set(battery_client_core_dir "${CMAKE_CURRENT_LIST_DIR}/battery-client-core")
set(POKELINUX_BATTERY_CLIENT_ARCHIVE "" CACHE FILEPATH "Explicit already-built matching battery client archive")
set(POKELINUX_BATTERY_CLIENT_ARCHIVE_SHA256 "" CACHE STRING "Required digest for an explicit battery client archive")
if((POKELINUX_BATTERY_CLIENT_ARCHIVE AND NOT POKELINUX_BATTERY_CLIENT_ARCHIVE_SHA256) OR (POKELINUX_BATTERY_CLIENT_ARCHIVE_SHA256 AND NOT POKELINUX_BATTERY_CLIENT_ARCHIVE))
 message(FATAL_ERROR "Explicit battery archive and SHA256 must be supplied together")
endif()
if(POKELINUX_BATTERY_CLIENT_ARCHIVE)
 file(SHA256 "${POKELINUX_BATTERY_CLIENT_ARCHIVE}" actual_battery_client_sha256)
 if(NOT actual_battery_client_sha256 STREQUAL POKELINUX_BATTERY_CLIENT_ARCHIVE_SHA256)
  message(FATAL_ERROR "Explicit battery client archive digest mismatch")
 endif()
 set(battery_client_archive "${POKELINUX_BATTERY_CLIENT_ARCHIVE}")
else()
 if(CMAKE_CROSSCOMPILING)
  message(FATAL_ERROR "Cross builds require an explicit pinned battery client archive for the matching target")
 endif()
 find_program(CARGO_EXECUTABLE cargo REQUIRED)
 set(battery_client_archive "${CMAKE_CURRENT_BINARY_DIR}/battery-rust-target/release/libpokelinux_battery_client.a")
 add_custom_target(battery-client-rust-build
  COMMAND ${CMAKE_COMMAND} -E env "CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/battery-rust-target"
   ${CARGO_EXECUTABLE} build --manifest-path "${battery_client_core_dir}/Cargo.toml" --locked --offline --release
  BYPRODUCTS "${battery_client_archive}"
  VERBATIM)
endif()
find_package(Threads REQUIRED)
add_library(battery-client-core STATIC IMPORTED GLOBAL)
set_target_properties(battery-client-core PROPERTIES
 IMPORTED_LOCATION "${battery_client_archive}"
 INTERFACE_INCLUDE_DIRECTORIES "${battery_client_core_dir}/include")
target_link_libraries(battery-client-core INTERFACE Threads::Threads ${CMAKE_DL_LIBS} m)
if(TARGET battery-client-rust-build)
 add_dependencies(battery-client-core battery-client-rust-build)
endif()
