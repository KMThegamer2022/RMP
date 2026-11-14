/* Generate the module index for the exact libraries shipped in the bundle. */
#include <stdlib.h>
extern void *libvlc_new(int, const char *const *);
extern void libvlc_release(void *);
int main(int argc,char **argv){
 if(argc!=2 || setenv("VLC_PLUGIN_PATH",argv[1],1))return 1;
 const char *args[]={"--ignore-config","--reset-plugins-cache"};
 void *vlc=libvlc_new(2,args);
 if(!vlc)return 1;
 libvlc_release(vlc);
 return 0;
}
