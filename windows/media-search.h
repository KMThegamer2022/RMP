/* Filename matches appear immediately. Search embedded tags off the UI thread
   after typing settles, including tracks that have never scrolled into view. */
enum { MEDIA_SEARCH_READY=WM_APP+45 };
typedef struct {
 HWND owner;LONG epoch;WCHAR query[256];size_t count;
 WCHAR **paths;bool *matches;
} TagSearch;
static volatile LONG searchEpoch;
static ULONGLONG searchDue;
static bool searchRunning;
static void updateMediaRows(const bool *extra,size_t count);
static void freeSearch(TagSearch *job){
 if(job->paths)for(size_t i=0;i<job->count;i++)free(job->paths[i]);
 free(job->paths);free(job->matches);free(job);
}
static DWORD WINAPI findTags(void *arg){
 TagSearch *job=arg;HRESULT co=CoInitializeEx(NULL,COINIT_APARTMENTTHREADED);
 if(SUCCEEDED(co)){
  for(size_t i=0;i<job->count;i++){
   if(InterlockedCompareExchange(&searchEpoch,0,0)!=job->epoch)break;
   if(!job->paths[i])continue;
   IPropertyStore *store=NULL;
   if(FAILED(SHGetPropertyStoreFromParsingName(job->paths[i],NULL,GPS_BESTEFFORT,&IID_IPropertyStore,(void**)&store)))continue;
   const PROPERTYKEY *keys[]={&PKEY_Title,&PKEY_Music_Artist,&PKEY_Music_AlbumTitle};
   for(int k=0;k<3&&!job->matches[i];k++){
    WCHAR *value=readMediaProperty(store,keys[k]);
    job->matches[i]=value&&containsTitle(value,job->query);CoTaskMemFree(value);
   }
   IPropertyStore_Release(store);
  }
  CoUninitialize();
 }
 if(!PostMessageW(job->owner,MEDIA_SEARCH_READY,0,(LPARAM)job))freeSearch(job);
 return 0;
}
static void beginTagSearch(void){
 if(!searchDue||searchRunning||GetTickCount64()<searchDue)return;
 searchDue=0;
 if(!IsWindowVisible(mediaFilter)||GetWindowTextLengthW(mediaFilter)==0)return;
 TagSearch *job=calloc(1,sizeof(*job));if(!job)return;
 job->epoch=InterlockedCompareExchange(&searchEpoch,0,0);job->owner=mainwin;
 GetWindowTextW(mediaFilter,job->query,256);
 bool library=browserPage==NAV_LIBRARY;
 job->count=library?ap_library_count(player):ap_count(player);
 job->matches=calloc(job->count,sizeof(*job->matches));job->paths=calloc(job->count,sizeof(*job->paths));
 if(!job->paths||!job->matches){freeSearch(job);return;}
 for(size_t i=0;i<job->count;i++){
  const char *source=library?ap_library_item(player,i):ap_item(player,i);
  if(!source||strstr(source,"://"))continue;
  WCHAR *path=wide(source);
  if(path&&!containsTitle(path,job->query))job->paths[i]=path;else free(path);
 }
 searchRunning=QueueUserWorkItem(findTags,job,WT_EXECUTEDEFAULT)!=0;
 if(!searchRunning)freeSearch(job);
}
static void receiveTagSearch(TagSearch *job){
 searchRunning=false;
 if(job->epoch==InterlockedCompareExchange(&searchEpoch,0,0)){
  for(size_t i=0;i<job->count;i++)if(job->matches[i]){updateMediaRows(job->matches,job->count);break;}
 }
 freeSearch(job);
}
