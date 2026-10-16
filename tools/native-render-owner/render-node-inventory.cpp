// Test-only metadata/capability probe for the explicitly selected guest DRM node.
#include <xf86drm.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <unistd.h>
#include <fcntl.h>
#include <algorithm>
#include <array>
#include <cerrno>
#include <cstdio>
#include <filesystem>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {
struct Fd { int value=-1; ~Fd(){if(value>=0)close(value);} };
struct VersionDeleter { void operator()(drmVersionPtr p)const { if(p)drmFreeVersion(p); } };
std::string safe(const std::string& text) {
    if(text.empty()||text.size()>128)throw std::runtime_error("name");
    for(unsigned char c:text)if(!((c>='a'&&c<='z')||(c>='A'&&c<='Z')||(c>='0'&&c<='9')||c=='_'||c=='-'))throw std::runtime_error("name");
    return text;
}
std::string ids(const std::array<unsigned,3>& values) {
    return "["+std::to_string(values[0])+","+std::to_string(values[1])+","+std::to_string(values[2])+"]";
}
}
int main(int argc,char** argv) {
    constexpr const char* NODE="/dev/dri/renderD128";
    std::array<uid_t,3> u{};std::array<gid_t,3> g{};
    const bool credentials=getresuid(&u[0],&u[1],&u[2])==0&&getresgid(&g[0],&g[1],&g[2])==0;
    const int groupCount=getgroups(0,nullptr);
    std::string body="{\"schema\":1,\"selected_node\":\"/dev/dri/renderD128\",\"actual_resuid\":"+ids(u)+",\"actual_resgid\":"+ids(g)+",\"actual_groups\":[";
    bool qualified=false;const char* stage="credentials";std::string inventory,actual;
    try {
        if(!credentials||groupCount<0||groupCount>32)throw std::runtime_error("credentials");
        std::vector<gid_t> groups(static_cast<size_t>(groupCount));
        if(groupCount&&getgroups(groupCount,groups.data())!=groupCount)throw std::runtime_error("groups");
        for(size_t i=0;i<groups.size();++i){if(i)body+=",";body+=std::to_string(groups[i]);}
        if(u!=std::array<uid_t,3>{1000,1000,1000}||g!=std::array<gid_t,3>{1000,1000,1000}||groupCount!=0)throw std::runtime_error("identity");
        stage="selected_node";if(argc!=3||std::string(argv[1])!="--node"||std::string(argv[2])!=NODE)throw std::runtime_error("selection");
        std::vector<std::filesystem::path> paths;
        for(const auto& entry:std::filesystem::directory_iterator("/dev/dri"))if(entry.path().filename().string().starts_with("renderD"))paths.push_back(entry.path());
        std::sort(paths.begin(),paths.end());if(paths.empty()||paths.size()>8)throw std::runtime_error("inventory cap");
        struct stat selected{};bool found=false;
        for(const auto& node:paths) {
            const auto basename=safe(node.filename().string());
            struct stat info{};if(lstat(node.c_str(),&info)||!S_ISCHR(info.st_mode))throw std::runtime_error("node type");
            auto driver=std::filesystem::path("/sys/dev/char")/(std::to_string(major(info.st_rdev))+":"+std::to_string(minor(info.st_rdev)))/"device/driver";
            const auto transport=safe(std::filesystem::canonical(driver).filename().string());
            if(!inventory.empty())inventory+=",";
            inventory+="{\"path\":\"/dev/dri/"+basename+"\",\"uid\":"+std::to_string(info.st_uid)+",\"gid\":"+std::to_string(info.st_gid)+",\"mode\":"+std::to_string(info.st_mode&07777)+",\"rdev\":"+std::to_string(info.st_rdev)+",\"major\":"+std::to_string(major(info.st_rdev))+",\"minor\":"+std::to_string(minor(info.st_rdev))+",\"transport_driver\":\""+transport+"\"}";
            if(node==NODE){if(found||major(info.st_rdev)!=226||minor(info.st_rdev)!=128||transport!="virtio-pci")throw std::runtime_error("transport identity");selected=info;found=true;}
        }
        if(!found)throw std::runtime_error("selected node absent");
        stage="original_UID_open";Fd fd;fd.value=open(NODE,O_RDWR|O_CLOEXEC|O_NOFOLLOW);if(fd.value<0)throw std::runtime_error("open");
        const int fdflags=fcntl(fd.value,F_GETFD);struct stat opened{};if(fstat(fd.value,&opened)||!S_ISCHR(opened.st_mode)||opened.st_rdev!=selected.st_rdev||fdflags<0||(fdflags&FD_CLOEXEC)==0)throw std::runtime_error("opened identity");
        stage="actual_DRM_version";
        std::unique_ptr<drmVersion,VersionDeleter> version(drmGetVersion(fd.value));
        if(!version||!version->name||version->name_len<=0||version->name_len>128)throw std::runtime_error("DRM version");
        const auto name=safe(std::string(version->name,static_cast<size_t>(version->name_len)));
        actual=",\"actual_opened_rdev\":"+std::to_string(opened.st_rdev)+",\"drm_driver\":\""+name+"\",\"DRM_version\":["+std::to_string(version->version_major)+","+std::to_string(version->version_minor)+","+std::to_string(version->version_patchlevel)+"]";
        if(name!="virtio_gpu")throw std::runtime_error("DRM driver mismatch");
        qualified=true;
    } catch(...) {}
    body+="],\"inventory\":["+inventory+"]"+actual+",\"no_permission_or_device_fallback\":true,\"raw_open_qualified\":"+(qualified?"true":"false");
    if(!qualified)body+=",\"setup_unavailable\":{\"stage\":\""+std::string(stage)+"\"}";
    body+="}\n";if(body.size()>16384)return 3;
    if(std::fwrite(body.data(),1,body.size(),stdout)!=body.size()||std::fflush(stdout))return 3;
    return qualified?0:2;
}
