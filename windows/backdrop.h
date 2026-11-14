/* Windows 10 exposes its blur policy through a dynamically resolved User32
   entry point. Prefer the documented system backdrop on newer Windows 11. */
typedef struct { int state, flags; DWORD tint; int animation; } AccentPolicy;
typedef struct { int attribute; void *data; SIZE_T size; } CompositionData;
static BOOL accentBlur(HWND w, bool enable){
 typedef BOOL (WINAPI *SetComposition)(HWND,CompositionData *);
 SetComposition set=(SetComposition)(void *)GetProcAddress(GetModuleHandleW(L"user32"),"SetWindowCompositionAttribute");
 AccentPolicy policy={enable?3:0,0,0,0};
 CompositionData data={19,&policy,sizeof(policy)};
 return set && set(w,&data);
}
static void updateGlass(HWND w){
 BOOL composed=FALSE;HIGHCONTRASTW contrast={sizeof(contrast)};
 SystemParametersInfoW(SPI_GETHIGHCONTRAST,sizeof(contrast),&contrast,0);
 contrastMode=(contrast.dwFlags&HCF_HIGHCONTRASTON)!=0;
 DwmIsCompositionEnabled(&composed);
 OSVERSIONINFOW os={sizeof(os)};GetVersionExW(&os);
 bool modern=os.dwMajorVersion>=10;
 DWORD transparency=1,size=sizeof(transparency);
 if(modern)RegGetValueW(HKEY_CURRENT_USER,L"Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",L"EnableTransparency",RRF_RT_REG_DWORD,NULL,&transparency,&size);
 bool wanted=skin==SEVEN && !fullscreen && composed && !contrastMode && transparency;
 glass=false;
 if(modern){
 int backdrop=wanted?3:1;
 accentBlur(w,false);
 HRESULT result=DwmSetWindowAttribute(w,38,&backdrop,sizeof(backdrop));
 if(wanted)glass=SUCCEEDED(result)||accentBlur(w,true);
 }else if(wanted && os.dwMajorVersion==6 && os.dwMinorVersion==1){glass=true;}
 MARGINS frame={0,0,0,glass?scale(100):0};
 if(composed && FAILED(DwmExtendFrameIntoClientArea(w,&frame))){
 glass=false;if(modern){int none=1;DwmSetWindowAttribute(w,38,&none,sizeof(none));accentBlur(w,false);}
 }
 InvalidateRect(w,NULL,TRUE);
}
