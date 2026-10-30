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

add_executable(generate_cellunion generate_cellunion.cc)
target_link_libraries(generate_cellunion PRIVATE s2::s2)

add_executable(generate_boolean_op generate_boolean_op.cc)
target_link_libraries(generate_boolean_op PRIVATE s2::s2)

add_executable(generate_coding generate_coding.cc)
target_link_libraries(generate_coding PRIVATE s2::s2)

add_executable(generate_values generate_values.cc)
target_link_libraries(generate_values PRIVATE s2::s2)

add_executable(generate_cap generate_cap.cc)
target_link_libraries(generate_cap PRIVATE s2::s2)

add_executable(generate_latlng generate_latlng.cc)
target_link_libraries(generate_latlng PRIVATE s2::s2)

add_executable(generate_region generate_region.cc)
target_link_libraries(generate_region PRIVATE s2::s2)

add_executable(generate_cell_bounds generate_cell_bounds.cc)
target_link_libraries(generate_cell_bounds PRIVATE s2::s2)
