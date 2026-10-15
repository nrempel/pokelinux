cmake_minimum_required(VERSION 3.22)
project(RestorationQtClient LANGUAGES CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_AUTOMOC ON)
find_package(Qt6 REQUIRED COMPONENTS Core)
set(MESSAGES_CONSUMER_DIR "${CMAKE_CURRENT_LIST_DIR}/../../../native-messages-ui"
    CACHE PATH "Frozen production MessagesClient source directory")
add_executable(messages-restoration-qt-client
    main.cpp
    "${MESSAGES_CONSUMER_DIR}/MessagesClient.cpp"
    "${MESSAGES_CONSUMER_DIR}/MessagesClient.h")
target_include_directories(messages-restoration-qt-client PRIVATE "${MESSAGES_CONSUMER_DIR}")
target_link_libraries(messages-restoration-qt-client PRIVATE Qt6::Core)
target_compile_options(messages-restoration-qt-client PRIVATE -Wall -Wextra -Werror)
file(WRITE "${CMAKE_CURRENT_BINARY_DIR}/qt-version.txt" "${Qt6Core_VERSION}\n")
