# Shared read-only Rust local network status client; Qt only presents copied state.
include_guard(GLOBAL)
set(network_status_client_core_dir "${CMAKE_CURRENT_LIST_DIR}/network-status-client-core")
set(POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE "" CACHE FILEPATH "Explicit already-built matching network status client archive")
set(POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE_SHA256 "" CACHE STRING "Required digest for an explicit network status client archive")
if((POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE AND NOT POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE_SHA256) OR (POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE_SHA256 AND NOT POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE))
 message(FATAL_ERROR "Explicit network status archive and SHA256 must be supplied together")
endif()
if(POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE)
 file(SHA256 "${POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE}" actual_network_status_client_sha256)
 if(NOT actual_network_status_client_sha256 STREQUAL POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE_SHA256)
  message(FATAL_ERROR "Explicit network status client archive digest mismatch")
 endif()
 set(network_status_client_archive "${POKELINUX_NETWORK_STATUS_CLIENT_ARCHIVE}")
else()
 if(CMAKE_CROSSCOMPILING)
  message(FATAL_ERROR "Cross builds require an explicit pinned network status client archive for the matching target")
 endif()
 find_program(CARGO_EXECUTABLE cargo REQUIRED)
 set(network_status_client_archive "${CMAKE_CURRENT_BINARY_DIR}/network-status-rust-target/release/libpokelinux_network_status_client.a")
 add_custom_target(network-status-client-rust-build
  COMMAND ${CMAKE_COMMAND} -E env "CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR}/network-status-rust-target"
   ${CARGO_EXECUTABLE} build --manifest-path "${network_status_client_core_dir}/Cargo.toml" --locked --offline --release
  BYPRODUCTS "${network_status_client_archive}"
  VERBATIM)
endif()
find_package(Threads REQUIRED)
add_library(network-status-client-core STATIC IMPORTED GLOBAL)
set_target_properties(network-status-client-core PROPERTIES
 IMPORTED_LOCATION "${network_status_client_archive}"
 INTERFACE_INCLUDE_DIRECTORIES "${network_status_client_core_dir}/include")
target_link_libraries(network-status-client-core INTERFACE Threads::Threads ${CMAKE_DL_LIBS} m)
if(TARGET network-status-client-rust-build)
 add_dependencies(network-status-client-core network-status-client-rust-build)
endif()
