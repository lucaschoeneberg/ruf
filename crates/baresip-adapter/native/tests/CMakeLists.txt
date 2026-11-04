cmake_minimum_required(VERSION 3.20)
project(sip_network_roam_test LANGUAGES C)

if(NOT SIP_SDK_PREFIX)
    message(FATAL_ERROR "Set SIP_SDK_PREFIX to the installed native SDK")
endif()

add_executable(network_roam_test network_roam_test.c ../network_roam.c)
target_compile_features(network_roam_test PRIVATE c_std_11)
target_include_directories(network_roam_test PRIVATE
    "${SIP_SDK_PREFIX}/include" "${SIP_SDK_PREFIX}/include/re")
find_library(RE_LIBRARY NAMES re PATHS "${SIP_SDK_PREFIX}/lib" NO_DEFAULT_PATH REQUIRED)
target_link_libraries(network_roam_test PRIVATE "${RE_LIBRARY}")

# These tests deliberately use assertions in every configuration.
if(MSVC)
    target_compile_options(network_roam_test PRIVATE /W4 /WX /UNDEBUG)
    target_compile_definitions(network_roam_test PRIVATE WIN32 _CRT_SECURE_NO_WARNINGS)
    target_link_libraries(network_roam_test PRIVATE ws2_32 iphlpapi dbghelp)
else()
    target_compile_options(network_roam_test PRIVATE -Wall -Wextra -Werror -UNDEBUG)
    if(UNIX AND NOT APPLE)
        target_link_libraries(network_roam_test PRIVATE m dl pthread resolv)
    endif()
endif()
