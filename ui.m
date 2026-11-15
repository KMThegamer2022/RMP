#import <Cocoa/Cocoa.h>
#import <AVFoundation/AVFoundation.h>
#include <stdbool.h>
#import "mac-theme.h"
#import "updates/mac.h"
typedef struct Player Player;
extern void ap_prepare(Player *);
extern void ap_stop(Player *);
extern Player *ap_create(void *);
extern void ap_destroy(Player *),ap_add(Player *,const char *,bool),ap_toggle(Player *),ap_seek(Player *,int64_t),ap_volume(Player *,int);
extern bool ap_play(Player *,size_t),ap_seekable(Player *),ap_select(Player *,bool,int);
extern long ap_index(Player *);
extern size_t ap_count(Player *);
extern const char *ap_item(Player *,size_t);
extern size_t ap_library_count(Player *);
extern const char *ap_library_item(Player *,size_t);
extern bool ap_library_play(Player *,size_t),ap_playlist_save(Player *,const char *),ap_playlist_load(Player *,const char *);
extern char *ap_terminal_command(Player *,const char *,int *);
extern char *ap_terminal_complete(Player *,const char *);
extern char *ap_terminal_prompt(Player *);
extern int ap_terminal_volume(Player *);
extern void ap_terminal_free(char *);
extern char *ap_menu_action(Player *,const char *,const char *);
extern char *ap_menu_options(Player *,const char *);
static NSString *takeTerminalString(char *);
extern bool ap_has_video(Player *);
extern int ap_state(Player *);
extern int64_t ap_time(Player *),ap_length(Player *);
extern void ap_tracks(Player *,bool,void *,void(*)(void *,int,const char *,bool));

@interface PlayerApp : NSObject<NSApplicationDelegate,NSMenuDelegate,NSWindowDelegate,NSTableViewDataSource,NSTableViewDelegate,NSSearchFieldDelegate>{
 Player *p;
 NSWindow *window,*terminalWindow;
 NSMenuItem *playMenuItem;
 NSScrollView *terminalScroll;NSLayoutConstraint *terminalTextHeight;NSTextView *terminalTranscript;NSTextField *terminalInput,*terminalPrompt;
 NSMutableArray<NSString *> *terminalHistory;NSUInteger terminalHistoryIndex;NSString *terminalDraft;
 NSView *idleBlack;
 NSImageView *pausedMark;NSImage *pausedRMP,*pausedCone;
 NSMapTable<NSMenu *,NSString *> *optionKeys;NSMenu *recentMenu;
 NSMutableDictionary *effectValues;
 BOOL quitAtEnd,recording,wallpaper,alwaysTop,minimalInterface,advancedVisible,statusVisible,playlistDocked,playlistShown;
 NSRect restoreFrame;NSWindowStyleMask restoreStyle;NSWindowCollectionBehavior restoreCollection;
 NSPanel *queuePanel;NSTableView *queueList;NSScrollView *queueScroll;NSView *dockedList;
 NSStackView *extraControls;NSTextField *statusLine;NSLayoutConstraint *bottomInset,*extraBottom;
 NSUInteger lastQueueCount;
 NSVisualEffectView *aeroDeck;
 NSView *screen;
 NSTextField *name,*status,*clockLabel,*welcome;
 NSSlider *seek,*volume;
 NSButton *play;
 NSMenu *queue,*audio,*subs,*themeMenu;
 NSInteger skin;BOOL inFull,repeatOne,shuffle;
 NSPanel *panel; NSButton *fp; NSSlider *fs,*fv;NSPoint lastPointer;double lastActivity;
 NSImageView *cone;NSStackView *top,*bottom,*timeline;
 NSLayoutConstraint *normalTop,*edgeTop,*normalBottom,*edgeBottom;

 MediaPane *navPane,*mediaHeader,*browserPane,*transportCapsule;
 NSCache *metadataCache;NSMutableSet *metadataPending;NSOperationQueue *metadataWorkers;NSMutableDictionary *searchNames;NSUInteger searchCursor;BOOL refillScheduled;
 NSSearchField *searchField;NSTableView *trackTable;NSScrollView *trackScroll;NSImageView *queueArt;NSLayoutConstraint *artHeight;
 NSTextField *crumb,*tableHeading;NSButton *headerOpen;NSArray<NSButton *> *navButtons;NSArray<NSNumber *> *mediaRows;
 BOOL pendingVideoSwitch;NSInteger mediaPage;NSLayoutConstraint *browserLeading,*browserWidth,*screenLeft,*screenRight,*mediaTop,*bottomLeft,*bottomRight,*bottomCenter;
 NSArray<NSView *> *originalTransport;
 NSMutableArray<NSURL *> *pending;
 NSTimer *timer;
 int previous;
}
@end

static NSString *stamp(int64_t ms){
 long s=MAX(0,ms)/1000;
 return s>=3600?[NSString stringWithFormat:@"%ld:%02ld:%02ld",s/3600,s/60%60,s%60]:[NSString stringWithFormat:@"%ld:%02ld",s/60,s%60];
}
static NSTextField *label(NSString *text,CGFloat size,BOOL bold){
 NSTextField *f=[NSTextField labelWithString:text];
 f.font=bold?[NSFont boldSystemFontOfSize:size]:[NSFont systemFontOfSize:size];
 f.lineBreakMode=NSLineBreakByTruncatingMiddle;
 return f;
}
@implementation PlayerApp
-(NSMenuItem *)item:(NSMenu *)m title:(NSString *)t action:(SEL)a key:(NSString *)key{
 NSMenuItem *i=[m addItemWithTitle:t action:a keyEquivalent:key];i.target=self;return i;
}
-(NSMenu *)menu:(NSString *)title parent:(NSMenu *)parent{
 NSMenuItem *item=[parent addItemWithTitle:title action:nil keyEquivalent:@""];
 NSMenu *m=[[NSMenu alloc]initWithTitle:title];item.submenu=m;return m;
}
-(void)manualUpdate:(id)sender{checkMacUpdates(YES);}
#include "mac-menus.h"
-(void)menus{
 NSMenu *bar=[NSMenu new];NSApp.mainMenu=bar;
 NSMenu *app=[self menu:@"RMP" parent:bar];
 [app addItemWithTitle:@"About RMP" action:@selector(orderFrontStandardAboutPanel:) keyEquivalent:@""];
 [self item:app title:@"Check for Updates…" action:@selector(manualUpdate:) key:@""];
 [app addItem:NSMenuItem.separatorItem];
 [app addItemWithTitle:@"Quit RMP" action:@selector(terminate:) keyEquivalent:@"q"];
 NSMenu *edit=[self menu:@"Edit" parent:bar];
 for(NSString *s in @[@"Cut",@"Copy",@"Paste",@"Select All"]){
 NSArray *keys=@[@"x",@"c",@"v",@"a"]; NSUInteger n=edit.numberOfItems;
 [edit addItemWithTitle:s action:NSSelectorFromString(@[@"cut:",@"copy:",@"paste:",@"selectAll:"][n]) keyEquivalent:keys[n]];
 }
 [self installPlayerMenus:bar];effectValues=[NSMutableDictionary new];
 themeMenu=[self menu:@"Theme" parent:[bar itemWithTitle:@"View"].submenu];
 for(NSString *title in @[@"VLC",@"RMP Default",@"Windows 10/11",@"Windows 7",@"Windows XP",@"Mac",@"Linux"]){
 NSMenuItem *entry=[self item:themeMenu title:title action:@selector(pickTheme:) key:@""];const NSInteger ids[]={VLC_THEME,RMP_DEFAULT,MODERN,SEVEN,XP,MAC,LINUX_THEME};entry.tag=ids[themeMenu.numberOfItems-1];
 }

}
-(NSButton *)button:(NSString *)title action:(SEL)action{
 TransportButton *b=[TransportButton buttonWithTitle:title target:self action:action];b.bordered=NO;return b;
}
-(void)styleView:(NSView *)v {
 Theme t=themes[skin];
 if([v isKindOfClass:TransportButton.class]){
 TransportButton *b=(TransportButton *)v;b.skin=skin;b.toolTip=b.title;b.font=[NSFont systemFontOfSize:13 weight:skin==MODERN?NSFontWeightMedium:NSFontWeightRegular];[b setNeedsDisplay:YES];
 }else if([v isKindOfClass:NSTextField.class] && v!=welcome){((NSTextField *)v).textColor=rgb(t.text);}
 if(v!=screen)for(NSView *child in v.subviews)[self styleView:child];
}
#include "mac-library.h"
#include "mac-terminal.h"
-(void)arrangeChrome {
 BOOL terminal=skin==LINUX_THEME, bare=inFull||terminal||wallpaper||minimalInterface;BOOL edge=bare||skin==VLC_THEME;
 mediaTop.active=NO;normalTop.active=NO;edgeTop.active=NO;normalBottom.active=NO;edgeBottom.active=NO;
 if(edge)edgeTop.active=YES;else if(![self hasMediaLayout])normalTop.active=YES;(bare?edgeBottom:normalBottom).active=YES;
 top.hidden=edge||[self hasMediaLayout];timeline.hidden=bare;bottom.hidden=bare;
 [self layoutMediaChrome];[self updateViewControls];
 cone.hidden=skin!=VLC_THEME||(p&&ap_index(p)>=0);welcome.hidden=YES;
}
/* Shows the RMP logo (VLC keeps its cone) while paused, stopped, or idle. */
-(void)syncPausedMark:(int)state{
 BOOL idle=!p||ap_index(p)<0;
 BOOL show=!screen.hidden && (state==4 || state==5 || (idle && skin!=VLC_THEME));
 if(skin==VLC_THEME && idle)show=NO;
 pausedMark.hidden=!show;
}
-(void)applyTheme:(NSInteger)value {
 NSInteger oldSkin=skin;
 skin=value>=0 && value<THEME_COUNT?value:VLC_THEME;
 if(oldSkin!=skin || !navButtons){pendingVideoSwitch=NO;mediaPage=skin==SEVEN?1:0;searchField.stringValue=@"";}
 if(originalTransport){
  NSArray *ordered=originalTransport;
  if(skin==SEVEN)ordered=@[originalTransport[7],originalTransport[6],originalTransport[2],originalTransport[1],play,originalTransport[3],originalTransport[10],originalTransport[4]];
  if(skin==XP)ordered=@[play,originalTransport[2],originalTransport[1],originalTransport[3],originalTransport[10],originalTransport[6],originalTransport[7],originalTransport[4]];
  if(![bottom.arrangedSubviews isEqualToArray:ordered]){
   for(NSView *v in bottom.arrangedSubviews){[bottom removeArrangedSubview:v];[v removeFromSuperview];}
   for(NSView *v in ordered)[bottom addArrangedSubview:v];
  }
  bottomLeft.active=NO;bottomRight.active=NO;bottomCenter.active=NO;
  if(skin==SEVEN)bottomCenter.active=YES;else{bottomLeft.active=YES;bottomRight.active=skin!=XP;}
 }
 ThemeSurface *surface=(ThemeSurface *)window.contentView;surface.skin=skin;[surface setNeedsDisplay:YES];
 BOOL translucent=skin==SEVEN && !NSWorkspace.sharedWorkspace.accessibilityDisplayShouldReduceTransparency;
 aeroDeck.hidden=!translucent;window.opaque=!translucent;
 window.appearance=[NSAppearance appearanceNamed:(skin==MAC||skin==RMP_DEFAULT||skin==LINUX_THEME)?NSAppearanceNameDarkAqua:NSAppearanceNameAqua];
 window.titlebarAppearsTransparent=YES;
 window.backgroundColor=translucent?NSColor.clearColor:rgb(skin==XP?0x3978db:skin==MODERN?0xf5f5f5:themes[skin].panel);
 [self styleView:surface];[self arrangeChrome];if(panel)[self styleView:panel.contentView];
 if(skin==XP || skin==MODERN){NSColor *ink=skin==XP?NSColor.whiteColor:rgb(0x181818);name.textColor=ink;status.textColor=ink;}
 screen.layer.borderWidth=skin==XP?4:skin==SEVEN?2:0;
 screen.layer.borderColor=rgb(skin==XP?0x1256ce:0x829db5).CGColor;
 ((VolumeSlider *)volume).wedge=skin==VLC_THEME;[volume setNeedsDisplay:YES];
 seek.trackFillColor=rgb(themes[skin].accent);volume.trackFillColor=rgb(themes[skin].accent);
 for(NSView *v in surface.subviews)if([v isKindOfClass:NSStackView.class] && [((NSStackView *)v).arrangedSubviews containsObject:play]){
 NSStackView *row=(NSStackView *)v;row.spacing=skin==SEVEN?2:skin==MODERN?18:8;
 }

 pausedMark.image=skin==VLC_THEME?pausedCone:pausedRMP;
 [self syncPausedMark:p?ap_state(p):0];
 idleBlack.hidden=skin!=LINUX_THEME || (p && ap_has_video(p) && ap_state(p)<=4);
 if(skin==LINUX_THEME){[self showTerminal:nil];if(panel)[panel orderOut:nil];}
 else if(oldSkin==LINUX_THEME){[terminalWindow orderOut:nil];[window makeKeyAndOrderFront:nil];if(inFull)[self showFloatingTransport];}
 for(NSMenuItem *entry in themeMenu.itemArray)entry.state=entry.tag==skin?NSControlStateValueOn:NSControlStateValueOff;
}
-(void)pickTheme:(NSMenuItem *)entry {
 if(entry.tag==LINUX_THEME && skin!=LINUX_THEME)[NSUserDefaults.standardUserDefaults setInteger:skin forKey:@"PreviousTheme"];
 [self applyTheme:entry.tag];[NSUserDefaults.standardUserDefaults setInteger:skin forKey:@"PlayerTheme"];
}
-(NSStackView *)row:(NSArray *)views{
 NSStackView *r=[NSStackView stackViewWithViews:views];r.spacing=10;r.orientation=NSUserInterfaceLayoutOrientationHorizontal;return r;
}
-(void)applicationDidFinishLaunching:(NSNotification *)note{
 [self menus];NSApp.appearance=[NSAppearance appearanceNamed:NSAppearanceNameDarkAqua];
 window=[[NSWindow alloc]initWithContentRect:NSMakeRect(0,0,900,570) styleMask:NSWindowStyleMaskTitled|NSWindowStyleMaskClosable|NSWindowStyleMaskResizable|NSWindowStyleMaskMiniaturizable backing:NSBackingStoreBuffered defer:NO];
 window.title=@"RMP";window.minSize=NSMakeSize(720,380);window.delegate=self;
 window.collectionBehavior=NSWindowCollectionBehaviorFullScreenPrimary;[window center];
 window.contentView=[[ThemeSurface alloc]initWithFrame:window.contentView.bounds];NSView *root=window.contentView;root.wantsLayer=YES;
 screen=[NSView new];screen.wantsLayer=YES;screen.layer.backgroundColor=NSColor.blackColor.CGColor;
 [NSEvent addLocalMonitorForEventsMatchingMask:NSEventMaskLeftMouseDown handler:^NSEvent *(NSEvent *event){
  if(event.window==self->window && !self->screen.hidden && event.clickCount==2){
   NSPoint point=[self->screen convertPoint:event.locationInWindow fromView:nil];
   if(NSPointInRect(point,self->screen.bounds)){[self full:nil];return nil;}
  }
  return event;
 }];
 cone=[NSImageView new];cone.image=[NSImage imageNamed:@"vlc-cone"];[screen addSubview:cone];cone.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[cone.centerXAnchor constraintEqualToAnchor:screen.centerXAnchor],[cone.centerYAnchor constraintEqualToAnchor:screen.centerYAnchor],[cone.widthAnchor constraintEqualToConstant:120],[cone.heightAnchor constraintEqualToConstant:120]]];
 welcome=label(@"RMP",42,YES);welcome.textColor=[NSColor colorWithWhite:0.65 alpha:1];
 [screen addSubview:welcome];welcome.translatesAutoresizingMaskIntoConstraints=NO;
 [welcome.centerXAnchor constraintEqualToAnchor:screen.centerXAnchor].active=YES;
 [welcome.centerYAnchor constraintEqualToAnchor:screen.centerYAnchor].active=YES;
 name=label(@"RMP",18,YES);status=label(@"Ready",12,NO);
 [name setContentCompressionResistancePriority:200 forOrientation:NSLayoutConstraintOrientationHorizontal];
 NSView *space=[NSView new];
 top=[self row:@[name,space,status,[self button:@"Open Files…" action:@selector(open:)]]];
 play=[self button:@"Play" action:@selector(toggle:)];
 volume=[VolumeSlider sliderWithValue:80 minValue:0 maxValue:100 target:self action:@selector(changeVolume:)];volume.toolTip=@"Volume";
 [volume.widthAnchor constraintEqualToConstant:85].active=YES;
 seek=[NSSlider sliderWithValue:0 minValue:0 maxValue:1 target:self action:@selector(scrub:)];seek.continuous=NO;seek.toolTip=@"Playback position";
 clockLabel=label(@"0:00 / 0:00",12,NO);clockLabel.font=[NSFont monospacedDigitSystemFontOfSize:12 weight:NSFontWeightRegular];
 timeline=[self row:@[seek,clockLabel]];
 bottom=[self row:@[play,[self button:@"Previous" action:@selector(prev:)],[self button:@"■" action:@selector(stopPlayback:)],[self button:@"Next" action:@selector(next:)],[self button:@"⛶" action:@selector(full:)],[self button:@"Queue" action:@selector(queuePopup:)],[self button:@"↻" action:@selector(repeatToggle:)],[self button:@"⤨" action:@selector(shuffleToggle:)],[NSView new],label(@"Volume",12,NO),volume]];
 originalTransport=[bottom.arrangedSubviews copy];
 for(NSView *v in @[top,screen,timeline,bottom]){
 [root addSubview:v];v.translatesAutoresizingMaskIntoConstraints=NO;
 CGFloat inset=v==screen?0:16;
 NSLayoutConstraint *leading=[v.leadingAnchor constraintEqualToAnchor:root.leadingAnchor constant:inset], *trailing=[v.trailingAnchor constraintEqualToAnchor:root.trailingAnchor constant:-inset];
 leading.active=YES;trailing.active=YES;
 if(v==screen){screenLeft=leading;screenRight=trailing;}if(v==bottom){bottomLeft=leading;bottomRight=trailing;bottomCenter=[bottom.centerXAnchor constraintEqualToAnchor:root.centerXAnchor];}
 }
 [top.topAnchor constraintEqualToAnchor:root.topAnchor constant:10].active=YES;
 normalTop=[screen.topAnchor constraintEqualToAnchor:top.bottomAnchor constant:10];edgeTop=[screen.topAnchor constraintEqualToAnchor:root.topAnchor];
 normalBottom=[timeline.topAnchor constraintEqualToAnchor:screen.bottomAnchor constant:6];edgeBottom=[screen.bottomAnchor constraintEqualToAnchor:root.bottomAnchor];
 [bottom.topAnchor constraintEqualToAnchor:timeline.bottomAnchor constant:6].active=YES;
 bottomInset=[bottom.bottomAnchor constraintEqualToAnchor:root.bottomAnchor constant:-12];bottomInset.active=YES;
 aeroDeck=[[NSVisualEffectView alloc]initWithFrame:NSZeroRect];aeroDeck.material=NSVisualEffectMaterialLight;
 aeroDeck.blendingMode=NSVisualEffectBlendingModeBehindWindow;aeroDeck.state=NSVisualEffectStateActive;
 [root addSubview:aeroDeck positioned:NSWindowBelow relativeTo:timeline];aeroDeck.translatesAutoresizingMaskIntoConstraints=NO;
 [NSLayoutConstraint activateConstraints:@[[aeroDeck.leadingAnchor constraintEqualToAnchor:root.leadingAnchor],[aeroDeck.trailingAnchor constraintEqualToAnchor:root.trailingAnchor],[aeroDeck.bottomAnchor constraintEqualToAnchor:root.bottomAnchor],[aeroDeck.topAnchor constraintEqualToAnchor:screen.bottomAnchor]]];
 idleBlack=[NSView new];idleBlack.wantsLayer=YES;idleBlack.layer.backgroundColor=NSColor.blackColor.CGColor;idleBlack.translatesAutoresizingMaskIntoConstraints=NO;
 [root addSubview:idleBlack positioned:NSWindowAbove relativeTo:screen];[NSLayoutConstraint activateConstraints:@[[idleBlack.leadingAnchor constraintEqualToAnchor:screen.leadingAnchor],[idleBlack.trailingAnchor constraintEqualToAnchor:screen.trailingAnchor],[idleBlack.topAnchor constraintEqualToAnchor:screen.topAnchor],[idleBlack.bottomAnchor constraintEqualToAnchor:screen.bottomAnchor]]];
 pausedMark=[NSImageView new];pausedMark.translatesAutoresizingMaskIntoConstraints=NO;pausedMark.imageScaling=NSImageScaleProportionallyUpOrDown;
 pausedRMP=[NSImage imageNamed:@"RMP"];if(!pausedRMP)pausedRMP=[[NSImage alloc]initWithContentsOfFile:[NSBundle.mainBundle pathForResource:@"RMP" ofType:@"icns"]];
 pausedCone=[NSImage imageNamed:@"vlc-cone"];pausedMark.hidden=YES;
 [root addSubview:pausedMark positioned:NSWindowAbove relativeTo:idleBlack];
 [NSLayoutConstraint activateConstraints:@[[pausedMark.centerXAnchor constraintEqualToAnchor:screen.centerXAnchor],[pausedMark.centerYAnchor constraintEqualToAnchor:screen.centerYAnchor],[pausedMark.widthAnchor constraintEqualToConstant:96],[pausedMark.heightAnchor constraintEqualToConstant:96]]];
 [self makeViewControls:root];[self buildMediaLayout:root];mediaTop=[screen.topAnchor constraintEqualToAnchor:mediaHeader.bottomAnchor];
 [NSUserDefaults.standardUserDefaults registerDefaults:@{@"PlayerTheme":@(VLC_THEME)}];
 [self applyTheme:[NSUserDefaults.standardUserDefaults integerForKey:@"PlayerTheme"]];
 p=ap_create((__bridge void *)screen);
 if(!p){NSAlert *a=[NSAlert new];a.messageText=@"RMP could not load its playback engine.";[a runModal];[NSApp terminate:nil];return;}
 if([NSUserDefaults.standardUserDefaults objectForKey:@"StartupVolume"]){volume.integerValue=[NSUserDefaults.standardUserDefaults integerForKey:@"StartupVolume"];[self changeVolume:volume];}
 [self rebuildMediaRows];[window makeKeyAndOrderFront:nil];[NSApp activateIgnoringOtherApps:YES];
 timer=[NSTimer scheduledTimerWithTimeInterval:0.2 target:self selector:@selector(refresh:) userInfo:nil repeats:YES];
 if(pending.count){[self accept:pending];pending=nil;}
 [self refresh:nil];if(skin==LINUX_THEME)[self showTerminal:nil];
 checkMacUpdates(NO);
}
-(void)application:(NSApplication *)app openURLs:(NSArray<NSURL *> *)urls{
 if(p)[self accept:urls];else{if(!pending)pending=[NSMutableArray new];[pending addObjectsFromArray:urls];}
}
-(void)accept:(NSArray<NSURL *> *)urls{
 size_t start=ap_count(p);
 for(NSURL *u in urls)ap_add(p,u.isFileURL?u.fileSystemRepresentation:u.absoluteString.UTF8String,!u.isFileURL);
 if(urls.count)[self launch:start];
}
-(NSString *)display:(size_t)i{
 NSString *s=[NSString stringWithUTF8String:ap_item(p,i)];
 return s.lastPathComponent.length?s.lastPathComponent:s;
}
-(void)launch:(size_t)i{
 if(i>=ap_count(p))return;
 [self showPlayingItem:i];[self rememberRecent:[NSString stringWithUTF8String:ap_item(p,i)]];
 status.stringValue=ap_play(p,i)?@"Opening…":@"Could not open media";
}
-(void)refresh:(id)sender{
 int state=ap_state(p);
 [self syncPausedMark:state];
 if(lastQueueCount!=ap_count(p)){lastQueueCount=ap_count(p);[queueList reloadData];}
 if(skin==LINUX_THEME)idleBlack.hidden=state>=1&&state<=4&&ap_has_video(p);
 if(pendingVideoSwitch && ap_has_video(p)){pendingVideoSwitch=NO;mediaPage=0;[self arrangeChrome];}
 if(state>=6)pendingVideoSwitch=NO;
 if(state==6 && previous!=6 && repeatOne){[self launch:ap_index(p)];return;}
 if(state==6 && previous!=6 && (shuffle||ap_index(p)+1<(long)ap_count(p))){[self next:nil];return;}
 if(state==6 && previous!=6 && quitAtEnd){[NSApp terminate:nil];return;}
 previous=state;
 NSArray *states=@[@"Ready",@"Opening…",@"Buffering…",@"Playing",@"Paused",@"Stopped",@"Finished",@"Playback error"];
 status.stringValue=states[MAX(0,MIN(7,state))];play.title=state>=1&&state<=3?@"Pause":@"Play";
 int64_t t=ap_time(p),d=ap_length(p);seek.enabled=ap_seekable(p)&&d>0;
 seek.doubleValue=d>0?(double)t/d:0;clockLabel.stringValue=[NSString stringWithFormat:@"%@ / %@",stamp(t),d>0?stamp(d):@"—"];statusLine.stringValue=[NSString stringWithFormat:@"%@   %@",status.stringValue,clockLabel.stringValue];[self refreshFloating];
}
-(void)open:(id)s{
 ap_prepare(p);
 NSOpenPanel *picker=[NSOpenPanel openPanel];picker.allowsMultipleSelection=YES;
 if([picker runModal]==NSModalResponseOK)[self accept:picker.URLs];
}
-(void)stream:(id)s{
 NSAlert *a=[NSAlert new];a.messageText=@"Open Network Stream";[a addButtonWithTitle:@"Open"];[a addButtonWithTitle:@"Cancel"];
 NSTextField *f=[[NSTextField alloc]initWithFrame:NSMakeRect(0,0,400,24)];f.placeholderString=@"https://";a.accessoryView=f;a.window.initialFirstResponder=f;
 if([a runModal]!=NSAlertFirstButtonReturn)return;
 NSURL *url=[NSURL URLWithString:[f.stringValue stringByTrimmingCharactersInSet:NSCharacterSet.whitespaceAndNewlineCharacterSet]];
 if(url.scheme.length && !url.isFileURL){[self accept:@[url]];return;}
 NSAlert *err=[NSAlert new];err.messageText=@"Enter a stream URL including its protocol.";[err runModal];
}
-(void)prev:(id)s{if(ap_index(p)>0)[self launch:ap_index(p)-1];else if(ap_count(p))[self launch:0];}
-(void)next:(id)s{NSUInteger n=ap_count(p);if(shuffle && n>1)[self launch:(ap_index(p)+1+arc4random_uniform((uint32_t)n-1))%n];else [self launch:ap_index(p)+1];}
-(void)repeatToggle:(NSButton *)b{repeatOne=!repeatOne;b.title=repeatOne?@"↻ 1":@"↻";b.toolTip=@"Repeat current item";}
-(void)shuffleToggle:(NSButton *)b{shuffle=!shuffle;b.title=shuffle?@"⤨ ✓":@"⤨";b.toolTip=@"Shuffle queue";}
-(void)toggle:(id)s{if(!ap_count(p))[self open:s];else ap_toggle(p);}
#include "mac-fullscreen.h"
-(void)stopPlayback:(id)s{ap_stop(p);}
-(void)queuePopup:(NSButton *)s{[queue popUpMenuPositioningItem:nil atLocation:NSMakePoint(0,s.bounds.size.height) inView:s];}
-(void)full:(id)s{if(wallpaper)[self toggleWallpaper];[window toggleFullScreen:nil];}
-(void)scrub:(id)s{ap_seek(p,seek.doubleValue*ap_length(p));}
-(void)changeVolume:(id)s{ap_volume(p,volume.intValue);}
-(void)choose:(NSMenuItem *)i{if(i.menu==queue)[self launch:i.tag];else if(!ap_select(p,i.menu==subs,(int)i.tag))NSBeep();}
static void addTrack(void *context,int idNum,const char *title,bool active){
 NSMenu *m=(__bridge NSMenu *)context; NSMenuItem *i=[m addItemWithTitle:[NSString stringWithUTF8String:title]?:@"Track" action:@selector(choose:) keyEquivalent:@""];
 i.target=m.delegate;i.tag=idNum;i.state=active?NSControlStateValueOn:NSControlStateValueOff;
}
-(void)menuNeedsUpdate:(NSMenu *)m{
 if([self populateMediaMenu:m])return;
 [m removeAllItems];if(!p)return;
 if(m==queue){for(size_t n=0;n<ap_count(p);n++)addTrack((__bridge void *)m,(int)n,[self display:n].UTF8String,(long)n==ap_index(p));}
 else ap_tracks(p,m==subs,(__bridge void *)m,addTrack);
 if(!m.numberOfItems)[m addItemWithTitle:m==queue?@"Queue is empty":@"No tracks available" action:nil keyEquivalent:@""];
}
-(BOOL)applicationShouldTerminateAfterLastWindowClosed:(NSApplication *)a{return YES;}
-(void)applicationWillTerminate:(NSNotification *)n{[timer invalidate];if(p)ap_destroy(p);}
@end
int main(void){@autoreleasepool{
 NSString *plugins=[NSBundle.mainBundle.executablePath.stringByDeletingLastPathComponent stringByAppendingPathComponent:@"plugins"];
 setenv("VLC_PLUGIN_PATH",plugins.fileSystemRepresentation,1);
 NSApplication *app=NSApplication.sharedApplication;[app setActivationPolicy:NSApplicationActivationPolicyRegular];
 __attribute__((objc_precise_lifetime)) PlayerApp *delegate=[PlayerApp new];app.delegate=delegate;[app run];(void)delegate;
}return 0;}
