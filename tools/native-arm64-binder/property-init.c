/* Fixture startup property through genuine retained Bionic; no library interposition. */
#include <sys/system_properties.h>
#include <stdio.h>
#include <dlfcn.h>
#include <string.h>
int main(void){
    const char* name="servicemanager.ready";
    int (*area_init)(void)=dlsym(RTLD_DEFAULT,"__system_property_area_init");
    if(!area_init)return 2;
    int init=area_init();
    int (*add_property)(const char*,unsigned,const char*,unsigned)=dlsym(RTLD_DEFAULT,"__system_property_add");
    if(!add_property)return 2;
    int add=add_property(name,strlen(name),"true",4);
    char value[PROP_VALUE_MAX]={0};int length=__system_property_get(name,value);
    printf("GENUINE-BIONIC-PROPERTY init=%d add=%d length=%d value=%s\n",init,add,length,value);
    /* Init can report absent SELinux xattr support in this Arch test guest.
       Property admission/readback must actually succeed; no policy qualification. */
    return add==0 && length==4 && strcmp(value,"true")==0 ? 0 : 1;
}
