#pragma once
#include <algorithm>
#include <chrono>
#include <cstdarg>
#include <cerrno>
#include <fcntl.h>
#include <fstream>
#include <iostream>
#include <sstream>
#include <string>
#include <unistd.h>
using namespace std::chrono_literals;
namespace android::base {
class unique_fd { int n=-1; public:unique_fd()=default;explicit unique_fd(int x):n(x){}~unique_fd(){reset();}unique_fd(const unique_fd&)=delete;unique_fd& operator=(const unique_fd&)=delete;void reset(int x=-1){if(n>=0)::close(n);n=x;}operator int()const{return n;} };
inline bool WriteStringToFd(const std::string& s,int fd){size_t pos=0;while(pos<s.size()){auto n=::write(fd,s.data()+pos,s.size()-pos);if(n<0&&errno==EINTR)continue;if(n<=0)return false;pos+=n;}return true;}
inline bool ReadFileToString(const std::string&p,std::string*out){std::ifstream in(p);if(!in)return false;std::ostringstream s;s<<in.rdbuf();*out=s.str();return true;}
inline bool GetBoolProperty(const std::string&,bool fallback){return fallback;}
inline std::string StringPrintf(const char*f,...){char b[8192];va_list a;va_start(a,f);vsnprintf(b,sizeof b,f,a);va_end(a);return b;}
inline std::string Trim(const std::string&s){auto lo=s.find_first_not_of(" \t\r\n");if(lo==std::string::npos)return "";return s.substr(lo,s.find_last_not_of(" \t\r\n")-lo+1);}
class Timer{std::chrono::steady_clock::time_point begin=std::chrono::steady_clock::now();public:auto duration(){return std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now()-begin);}};
}
#define LOG(level) std::cerr
#define ATRACE_TAG_POWER 0
#define ATRACE_TAG_HAL 0
#define ATRACE_ENABLED() false
#define ATRACE_INT(...) ((void)0)
#define ATRACE_BEGIN(...) ((void)0)
#define ATRACE_END(...) ((void)0)
#ifndef TEMP_FAILURE_RETRY
#define TEMP_FAILURE_RETRY(expr) (expr)
#endif
