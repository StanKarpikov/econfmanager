cmake_minimum_required(VERSION 3.10)
project(peripheral_service C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

find_package(SQLite3 REQUIRED)

set(LIBRARY_OUTPUT_PATH ${CMAKE_SOURCE_DIR}/lib)
set(ECONF_LIB ${LIBRARY_OUTPUT_PATH}/libeconfmanager.a)
set(ECONF_INCLUDE ${LIBRARY_OUTPUT_PATH})

if(NOT EXISTS ${ECONF_LIB})
    message(FATAL_ERROR "Rust library not found: ${ECONF_LIB}")
endif()

if(NOT EXISTS ${ECONF_INCLUDE}/econfmanager.h)
    message(FATAL_ERROR "Header file not found: ${ECONF_INCLUDE}/econfmanager.h")
endif()

include_directories(
    ${CMAKE_SOURCE_DIR}
    ${ECONF_INCLUDE}
)

set(SOURCES
    service.c
)

add_executable(peripheral_service ${SOURCES})

target_link_libraries(peripheral_service
    ${ECONF_LIB}
    # System libraries needed by Rust and Econfmanager
    ${SQLite3_LIBRARIES}
    dl
    rt
    pthread
    m
)

set(EXECUTABLE_OUTPUT_PATH ${CMAKE_BINARY_DIR}/bin)