#import "themes.h"
static NSColor *rgb(unsigned v) {
 return [NSColor colorWithSRGBRed:(v>>16)/255.0 green:((v>>8)&255)/255.0 blue:(v&255)/255.0 alpha:1];
}
@interface ThemeSurface:NSView
@property NSInteger skin;
@end
@implementation ThemeSurface
-(void)drawRect:(NSRect)dirty {
 Theme t=themes[self.skin];
 if(self.skin==SEVEN){
 NSGradient *g=[[NSGradient alloc]initWithColors:@[rgb(t.panel),rgb(0xf2faff),rgb(0xa2c6e3),rgb(t.panel)]];
 [g drawInRect:self.bounds angle:90];
 }else{[rgb(t.panel)setFill];NSRectFill(dirty);}
}
@end
@interface TransportButton:NSButton
@property NSInteger skin;
@end
@implementation TransportButton
-(NSSize)intrinsicContentSize {NSSize s=super.intrinsicContentSize;s.width+=14;s.height=30;return s;}
-(void)drawRect:(NSRect)dirty {
 Theme t=themes[self.skin];NSRect r=NSInsetRect(self.bounds,1.5,2);
 NSBezierPath *path=[NSBezierPath bezierPathWithRoundedRect:r xRadius:t.radius yRadius:t.radius];
 BOOL pressed=self.cell.isHighlighted;
 [NSGraphicsContext saveGraphicsState];[path addClip];
 if(self.skin==XP || self.skin==SEVEN){
 NSGradient *g=[[NSGradient alloc]initWithColors:@[rgb(t.highlight),rgb(t.face),rgb(pressed?t.edge:t.face)]];
 [g drawInRect:r angle:90];
 }else{[rgb(pressed?t.edge:t.face)setFill];[path fill];}
 [NSGraphicsContext restoreGraphicsState];[rgb(t.edge)setStroke];[path stroke];
 NSDictionary *attrs=@{NSFontAttributeName:self.font,NSForegroundColorAttributeName:rgb(t.text)};
 NSSize size=[self.title sizeWithAttributes:attrs];
 [self.title drawAtPoint:NSMakePoint(NSMidX(r)-size.width/2,NSMidY(r)-size.height/2) withAttributes:attrs];
 if(self.window.firstResponder==self){[rgb(t.accent)setStroke];path.lineWidth=2;[path stroke];}
}
@end
