cmake_minimum_required(VERSION 3.16)
project(refgen_cpp CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Requires an installed google/s2geometry (and its abseil). Point
# CMAKE_PREFIX_PATH at the install prefix — see README.md:
#   cmake -B build -DCMAKE_PREFIX_PATH=/path/to/s2-install
find_package(s2 CONFIG REQUIRED)

add_executable(generate_cellid generate_cellid.cc)
target_link_libraries(generate_cellid PRIVATE s2::s2)
