// Our harness. Implementation and headers are fetched separately, unmodified.
#include "perfmgr/FileNode.h"
#include <filesystem>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <string>
using namespace android::perfmgr;
using namespace std::chrono_literals;
static void row(const std::string& scope,const std::string& key,const std::string& value) {
    std::cout << scope << '\t' << key << '\t' << value << '\n';
}
static void fact(const std::string& key,bool value) { row("shared",key,value?"1":"0"); }
static std::string read(const std::string& p) { std::ifstream in(p);return {std::istreambuf_iterator<char>(in),{}}; }
static size_t held(const std::string& p) {
    size_t n=0;
    for(auto& e:std::filesystem::directory_iterator("/proc/self/fd")) {
        std::error_code error;auto link=std::filesystem::read_symlink(e.path(),error);
        if(!error&&link==p)++n;
    }
    return n;
}
int main(int argc,char** argv) {
    if(argc!=2)throw std::runtime_error("one private temporary directory required");
    auto root=std::filesystem::absolute(argv[1]);
    if(!std::filesystem::create_directory(root))throw std::runtime_error("fresh directory required");
    struct Cleanup { std::filesystem::path path;~Cleanup(){std::filesystem::remove_all(path);} } cleanup{root};
    auto now=std::chrono::steady_clock::now();std::chrono::milliseconds expiry;
    RequestGroup group("synthetic");
    group.AddRequest("owner",now+1h);group.AddRequest("owner",now+2h);
    fact("renewal-extended",group.GetExpireTime(&expiry)&&expiry>7100s);
    group.AddRequest("owner",now+30min);
    fact("renewal-never-shortens",group.GetExpireTime(&expiry)&&expiry>7100s);
    group.RemoveRequest("owner");group.AddRequest("past",now-1s);
    fact("expired-active",group.GetExpireTime(&expiry));
    group.AddRequest("owner",ReqTime::max());
    fact("forever-active",group.GetExpireTime(&expiry)&&expiry>24h);
    fact("owner-cancel",group.RemoveRequest("owner"));
    fact("cancelled-active",group.GetExpireTime(&expiry));
    std::vector<RequestGroup> values={RequestGroup("high"),RequestGroup("low"),RequestGroup("idle")};
    auto p=(root/"node").string();std::ofstream(p)<<"stale";
    FileNode node("synthetic",p,values,2,true,true,true);
    node.Update(false);row("shared","initial-default",read(p));
    node.AddRequest(1,"low-owner",ReqTime::max());node.Update(false);row("shared","low-value",read(p));
    node.AddRequest(0,"high-owner",ReqTime::max());node.Update(false);row("shared","high-value",read(p));
    row("reference-only","held-descriptor",std::to_string(held(p)));
    node.RemoveRequest("high-owner");node.Update(false);row("shared","cancel-high",read(p));
    node.RemoveRequest("low-owner");node.Update(false);row("shared","cancel-last",read(p));
    row("reference-only","released-descriptor",std::to_string(held(p)));
    node.AddRequest(0,"expired",now-1s);node.Update(false);row("shared","expired-high",read(p));
    fact("invalid-value-accepted",node.AddRequest(3,"bad-index",ReqTime::max()));
    node.AddRequest(1,"same-owner",ReqTime::max());node.AddRequest(0,"same-owner",ReqTime::max());
    node.RemoveRequest("same-owner");node.Update(false);row("shared","cancel-owner-all-values",read(p));
    auto missing=(root/"missing-node").string();FileNode retry("retry",missing,values,2,false,true);
    retry.AddRequest(0,"owner",ReqTime::max());
    row("shared","failed-retry-ms",std::to_string(retry.Update(false).count()));
    fact("failed-file-absent",!std::filesystem::exists(missing));
    std::ofstream(missing)<<"old";retry.Update(false);row("shared","retry-current-value",read(missing));
    retry.RemoveRequest("owner");retry.Update(false);row("shared","retry-default",read(missing));
}
