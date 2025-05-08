cmake_minimum_required(VERSION 3.10)
project(peripheral_service C CXX)

find_package(SQLite3 REQUIRED)

set(LIBRARY_OUTPUT_PATH ${CMAKE_SOURCE_DIR}/lib)
set(ECONFMANAGER_DIR ${LIBRARY_OUTPUT_PATH})
find_library(ECONF_LIB econfmanager PATHS ${ECONFMANAGER_DIR} REQUIRED)

if(NOT EXISTS ${ECONFMANAGER_DIR}/econfmanager.h)
    message(FATAL_ERROR "Header file not found: ${ECONFMANAGER_DIR}/econfmanager.h")
endif()

include_directories(
    ${CMAKE_SOURCE_DIR}
    ${ECONFMANAGER_DIR}
)

set(SOURCES
    service.c
)

add_executable(${PROJECT_NAME} ${SOURCES})

target_link_directories(${PROJECT_NAME} PRIVATE
    ${ECONFMANAGER_DIR})

target_link_libraries(${PROJECT_NAME}
    ${ECONF_LIB}
    # System libraries needed by Rust and Econfmanager
    ${SQLite3_LIBRARIES}
    dl
    rt
    pthread
    m
)

set(EXECUTABLE_OUTPUT_PATH ${CMAKE_BINARY_DIR}/bin)