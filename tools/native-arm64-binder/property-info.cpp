// Test-only property metadata through pinned original public AOSP serializer.
#include <property_info_serializer/property_info_serializer.h>
#include <fstream>
#include <cstdio>
int main(int argc,char** argv){
    if(argc!=2)return 2;
    std::string bytes,error;
    const char* context="u:object_r:default_prop:s0";
    if(!android::properties::BuildTrie({{"servicemanager.ready",context,"bool",true}},context,"string",&bytes,&error)){fprintf(stderr,"%s\n",error.c_str());return 1;}
    std::ofstream out(argv[1],std::ios::binary);out.write(bytes.data(),bytes.size());out.close();return out?0:1;
}
