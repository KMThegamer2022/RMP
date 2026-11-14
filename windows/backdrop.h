enum { STANDARD_TRANSPORT_HEIGHT = 100, MEDIA_TRANSPORT_HEIGHT = 79 };

static void updateGlass(HWND window){
 HIGHCONTRASTW contrast={sizeof(contrast)};
 SystemParametersInfoW(SPI_GETHIGHCONTRAST,sizeof(contrast),&contrast,0);
 contrastMode=(contrast.dwFlags&HCF_HIGHCONTRASTON)!=0;
 BOOL composed=FALSE;
 DwmIsCompositionEnabled(&composed);
 bool extend=skin==SEVEN && !fullscreen && composed && !contrastMode;
 MARGINS margins={0,0,0,extend?scale(MEDIA_TRANSPORT_HEIGHT):0};
 HRESULT result=DwmExtendFrameIntoClientArea(window,&margins);
 glass=extend && SUCCEEDED(result);
 InvalidateRect(window,NULL,TRUE);
}
