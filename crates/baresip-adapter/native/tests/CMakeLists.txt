cmake_minimum_required(VERSION 3.20)
project(sip_network_roam_test LANGUAGES C)

if(NOT SIP_SDK_PREFIX)
    message(FATAL_ERROR "Set SIP_SDK_PREFIX to the installed native SDK")
endif()

add_executable(network_roam_test network_roam_test.c ../network_roam.c)
target_compile_features(network_roam_test PRIVATE c_std_11)
target_include_directories(network_roam_test SYSTEM PRIVATE
    "${SIP_SDK_PREFIX}/include" "${SIP_SDK_PREFIX}/include/re")
find_library(RE_LIBRARY NAMES re PATHS "${SIP_SDK_PREFIX}/lib" NO_DEFAULT_PATH REQUIRED)
target_link_libraries(network_roam_test PRIVATE "${RE_LIBRARY}")

if(APPLE)
    add_executable(dns_refresh_test dns_refresh.c)
    target_compile_features(dns_refresh_test PRIVATE c_std_11)
    target_compile_options(dns_refresh_test PRIVATE -Wall -Wextra -Werror -UNDEBUG)
    target_include_directories(dns_refresh_test PRIVATE "${SIP_SDK_PREFIX}/include/re")
    target_link_libraries(dns_refresh_test PRIVATE "${RE_LIBRARY}" resolv
        "-framework CoreFoundation" "-framework SystemConfiguration")
endif()

# These tests deliberately use assertions in every configuration.
if(MSVC)
    target_compile_options(network_roam_test PRIVATE /W4 /WX)
    target_compile_definitions(network_roam_test PRIVATE WIN32 _CRT_SECURE_NO_WARNINGS)
    target_link_libraries(network_roam_test PRIVATE ws2_32 iphlpapi dbghelp)
else()
    target_compile_options(network_roam_test PRIVATE -Wall -Wextra -Werror)
    if(UNIX AND NOT APPLE)
        target_link_libraries(network_roam_test PRIVATE m dl pthread resolv)
    endif()
endif()

add_executable(call_actions_test call_actions.c ../audio_diagnostics.c ../security.c)
add_executable(latency_probe ../latency_probe.c)
foreach(test call_actions_test latency_probe)
    target_compile_features(${test} PRIVATE c_std_11)
    target_include_directories(${test} SYSTEM PRIVATE
        "${SIP_SDK_PREFIX}/include" "${SIP_SDK_PREFIX}/include/re")
    foreach(library baresip re ssl crypto)
        find_library(SIP_${library}_LIBRARY NAMES ${library}
            PATHS "${SIP_SDK_PREFIX}/lib" NO_DEFAULT_PATH REQUIRED)
        target_link_libraries(${test} PRIVATE "${SIP_${library}_LIBRARY}")
    endforeach()
    if(MSVC)
        target_compile_options(${test} PRIVATE /W4 /WX)
        target_compile_definitions(${test} PRIVATE WIN32 _CRT_SECURE_NO_WARNINGS)
        target_link_libraries(${test} PRIVATE qwave iphlpapi wsock32 ws2_32
            dbghelp winmm gdi32 crypt32 strmiids ole32 oleaut32 bcrypt ncrypt advapi32
            user32 uuid avrt)
    else()
        target_compile_options(${test} PRIVATE -Wall -Wextra -Werror)
        if(APPLE)
            foreach(framework CoreFoundation SystemConfiguration CoreAudio AudioToolbox Security)
                target_link_libraries(${test} PRIVATE "-framework ${framework}")
            endforeach()
            target_link_libraries(${test} PRIVATE resolv)
        else()
            target_link_libraries(${test} PRIVATE m dl pthread resolv)
        endif()
    endif()
endforeach()
