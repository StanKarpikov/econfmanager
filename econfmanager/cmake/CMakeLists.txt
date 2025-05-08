cmake_minimum_required(VERSION 3.12)
project(EconfManager LANGUAGES C)

add_library(EconfManager INTERFACE)

target_include_directories(EconfManager INTERFACE
    ${CMAKE_CURRENT_SOURCE_DIR}
)