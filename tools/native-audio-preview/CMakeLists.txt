cmake_minimum_required(VERSION 3.22)
project(sounds_preview_qml_test LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(POKELINUX_SOURCE_ROOT "" CACHE PATH "Selected production source root")
set(PREVIEW_QT_VERSION "" CACHE STRING "Exact selected Qt version")
set(SOUNDS_ARCHIVE "" CACHE FILEPATH "Selected target Sounds static archive")
set(SOUNDS_ARCHIVE_SHA256 "" CACHE STRING "Expected Sounds archive SHA256")
set(APPEARANCE_ARCHIVE "" CACHE FILEPATH "Selected target appearance static archive")
set(APPEARANCE_ARCHIVE_SHA256 "" CACHE STRING "Expected appearance archive SHA256")
if(NOT POKELINUX_SOURCE_ROOT OR NOT PREVIEW_QT_VERSION)
 message(FATAL_ERROR "Explicit production source root and selected Qt version required")
endif()
foreach(role SOUNDS APPEARANCE)
 if(NOT EXISTS "${${role}_ARCHIVE}")
  message(FATAL_ERROR "Explicit existing ${role}_ARCHIVE required; no Cargo fallback")
 endif()
 string(LENGTH "${${role}_ARCHIVE_SHA256}" hash_length)
 if(NOT hash_length EQUAL 64 OR NOT "${${role}_ARCHIVE_SHA256}" MATCHES "^[0-9a-f]+$")
  message(FATAL_ERROR "Explicit lowercase ${role}_ARCHIVE_SHA256 required")
 endif()
 file(SHA256 "${${role}_ARCHIVE}" actual)
 if(NOT actual STREQUAL "${${role}_ARCHIVE_SHA256}")
  message(FATAL_ERROR "${role}_ARCHIVE SHA256 mismatch")
 endif()
endforeach()
find_package(Qt6 ${PREVIEW_QT_VERSION} EXACT REQUIRED COMPONENTS Core Gui Network Qml Quick QuickControls2 Test)
find_package(Threads REQUIRED)
add_library(sounds-core STATIC IMPORTED)
set_target_properties(sounds-core PROPERTIES IMPORTED_LOCATION "${SOUNDS_ARCHIVE}")
add_library(appearance-core STATIC IMPORTED)
set_target_properties(appearance-core PROPERTIES IMPORTED_LOCATION "${APPEARANCE_ARCHIVE}")
add_executable(sounds-qml-input-client qml-input-client.cpp
 "${POKELINUX_SOURCE_ROOT}/native-audio-ui/AudioClient.cpp"
 "${POKELINUX_SOURCE_ROOT}/native-audio-ui/AudioClient.h"
 "${POKELINUX_SOURCE_ROOT}/native-app/PhoneAppearanceClient.cpp"
 "${POKELINUX_SOURCE_ROOT}/native-app/PhoneAppearanceClient.h"
 "${POKELINUX_SOURCE_ROOT}/native-audio-ui/resources.qrc")
target_include_directories(sounds-qml-input-client PRIVATE
 "${POKELINUX_SOURCE_ROOT}/native-audio-ui" "${POKELINUX_SOURCE_ROOT}/native-app"
 "${POKELINUX_SOURCE_ROOT}/native-audio-ui/client-core/include"
 "${POKELINUX_SOURCE_ROOT}/native-app/appearance-client-core/include")
target_compile_definitions(sounds-qml-input-client PRIVATE _GNU_SOURCE)
target_compile_options(sounds-qml-input-client PRIVATE -Wall -Wextra -Werror -UNDEBUG)
target_link_options(sounds-qml-input-client PRIVATE -Wl,--wrap=send)
target_link_libraries(sounds-qml-input-client PRIVATE Qt6::Core Qt6::Gui Qt6::Network Qt6::Qml
 Qt6::Quick Qt6::QuickControls2 Qt6::Test sounds-core appearance-core Threads::Threads ${CMAKE_DL_LIBS} m)
# No install, automatic CTest, child launch, runtime setup or execution target.
