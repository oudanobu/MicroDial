import React, { useEffect, useState } from 'react';
import { calculateAltitude } from '../lib/calc';
import { SensorData } from '../types';
import { Language, translations } from '../lib/translations';
import { GeometrySettings } from './GeometryControls';
import { 
  Compass, Footprints, MapPin, Gauge, CloudRain, Mountain, 
  Terminal, History, Info, Cpu, CheckCircle2, ChevronRight, Activity 
} from 'lucide-react';

interface WatchFaceProps {
  sensorData: SensorData;
  lang: Language;
  geometry: GeometrySettings;
  onGeometryChange?: (geometry: GeometrySettings) => void;
}

export function WatchFace({ sensorData, lang, geometry, onGeometryChange }: WatchFaceProps) {
  const [time, setTime] = useState(new Date());
  const t = translations[lang];

  useEffect(() => {
    const timer = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(timer);
  }, []);

  const altitude = calculateAltitude(sensorData.pressure);
  const formattedAltitude = Math.max(0, Math.round(altitude));

  const dateStr = lang === 'zh'
    ? `${time.toLocaleDateString('zh-CN', { month: 'long', day: 'numeric' })} ${time.toLocaleDateString('zh-CN', { weekday: 'long' })}`
    : time.toLocaleDateString('en-US', { weekday: 'short', month: 'short', day: 'numeric' });

  // Calculate sliding frame viewports using the exact Rust algorithms requested:
  // let (face_x, drawer_x) = calculate_drawer_viewport(&geo, &touch);
  const maxOffset = -geometry.resolution;
  const clampedOffset = Math.max(maxOffset, Math.min(0, geometry.dragOffsetX));
  
  const faceX = clampedOffset;
  const drawerX = clampedOffset + geometry.resolution;

  // Render scale for low-res (240x240 fits neatly in a 320px physical container container)
  const isMini = geometry.resolution === 240;
  const scaleClass = isMini ? 'scale-75 origin-center' : 'scale-100';

  // Drag listeners to allow user to drag directly on the screen!
  const handleMouseDown = (e: React.MouseEvent) => {
    if (!onGeometryChange) return;
    onGeometryChange({
      ...geometry,
      isDragging: true,
      dragStartX: e.clientX - geometry.dragOffsetX
    });
  };

  const handleMouseMove = (e: React.MouseEvent) => {
    if (!geometry.isDragging || !onGeometryChange) return;
    const offset = e.clientX - geometry.dragStartX;
    const clamped = Math.max(-geometry.resolution, Math.min(0, offset));
    onGeometryChange({
      ...geometry,
      dragOffsetX: clamped
    });
  };

  const handleMouseUpOrLeave = () => {
    if (!geometry.isDragging || !onGeometryChange) return;
    
    // Snap feature: if swiped more than half, snap to drawer, else snap back to dial
    const halfRes = -geometry.resolution / 2;
    const finalOffset = geometry.dragOffsetX < halfRes ? -geometry.resolution : 0;
    
    onGeometryChange({
      ...geometry,
      isDragging: false,
      dragOffsetX: finalOffset
    });
  };

  // Mobile Touch handlers
  const handleTouchStart = (e: React.TouchEvent) => {
    if (!onGeometryChange || e.touches.length === 0) return;
    onGeometryChange({
      ...geometry,
      isDragging: true,
      dragStartX: e.touches[0].clientX - geometry.dragOffsetX
    });
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    if (!geometry.isDragging || !onGeometryChange || e.touches.length === 0) return;
    const offset = e.touches[0].clientX - geometry.dragStartX;
    const clamped = Math.max(-geometry.resolution, Math.min(0, offset));
    onGeometryChange({
      ...geometry,
      dragOffsetX: clamped
    });
  };

  return (
    <div className="relative flex flex-col items-center">
      
      {/* Absolute Physical Shell */}
      <div 
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUpOrLeave}
        onMouseLeave={handleMouseUpOrLeave}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleMouseUpOrLeave}
        style={{
          width: `${geometry.resolution + 24}px`,
          height: `${geometry.resolution + 24}px`,
        }}
        className={`relative transition-all duration-300 border-[12px] border-slate-800 shadow-2xl overflow-hidden ring-4 ring-slate-900 bg-black cursor-grab active:cursor-grabbing select-none ${
          geometry.shape === 'Round' ? 'rounded-full' : 'rounded-3xl'
        }`}
      >
        
        {/* Core Screen Frame - exact resolution mapped */}
        <div 
          className="relative w-full h-full overflow-hidden"
          style={{
            width: `${geometry.resolution}px`,
            height: `${geometry.resolution}px`,
          }}
        >
          {/* Viewport 1: Sports Watch Face Dial (faceX offset) */}
          <div 
            className="absolute top-0 left-0 w-full h-full flex flex-col items-center justify-center text-slate-100 bg-slate-950 transition-transform duration-75"
            style={{ 
              transform: `translateX(${faceX}px)`,
              width: `${geometry.resolution}px`,
              height: `${geometry.resolution}px`,
            }}
          >
            {/* Time Display */}
            <div className="mt-2 flex flex-col items-center">
              <span className={`font-mono font-bold tracking-tight text-white leading-none ${
                isMini ? 'text-4xl' : 'text-5xl'
              }`}>
                {time.getHours().toString().padStart(2, '0')}:
                {time.getMinutes().toString().padStart(2, '0')}
              </span>
              <span className={`font-sans text-slate-400 font-medium uppercase tracking-wide ${
                isMini ? 'text-[9px] mt-1' : 'text-xs mt-1.5'
              }`}>
                {dateStr}
              </span>
            </div>

            {/* Primary Metrics */}
            <div className={`flex mt-3 ${isMini ? 'gap-6' : 'gap-9'}`}>
              <div className="flex flex-col items-center justify-center">
                <Footprints className={`${isMini ? 'w-4 h-4' : 'w-5 h-5'} text-emerald-400 mb-0.5`} />
                <span className={`font-mono font-medium ${isMini ? 'text-sm' : 'text-lg'}`}>{sensorData.steps}</span>
                <span className="text-[10px] text-slate-500 font-medium">{t.steps}</span>
              </div>
              <div className="flex flex-col items-center justify-center">
                <Mountain className={`${isMini ? 'w-4 h-4' : 'w-5 h-5'} text-sky-400 mb-0.5`} />
                <span className={`font-mono font-medium ${isMini ? 'text-sm' : 'text-lg'}`}>
                  {formattedAltitude}{lang === 'zh' ? '米' : 'm'}
                </span>
                <span className="text-[10px] text-slate-500 font-medium">{lang === 'zh' ? '海拔' : 'Alt'}</span>
              </div>
            </div>

            {/* Weather & Location - Adjusted for low resolution scale */}
            <div className="absolute bottom-5 flex flex-col items-center w-full px-6 font-mono text-slate-400">
              <div className={`flex justify-between w-full mb-1 ${isMini ? 'text-[9px]' : 'text-xs'}`}>
                <div className="flex items-center gap-1">
                  <Gauge className="w-3 h-3 text-slate-500" />
                  <span>{sensorData.pressure.toFixed(1)} hPa</span>
                </div>
                <div className="flex items-center gap-1">
                  <CloudRain className="w-3 h-3 text-slate-500" />
                  {sensorData.weather ? (
                    <span>{sensorData.weather.temperature}°C</span>
                  ) : (
                    <span>--</span>
                  )}
                </div>
              </div>
              
              <div className="flex items-center gap-1 text-[10px] justify-center text-slate-500">
                <MapPin className="w-2.5 h-2.5 text-slate-500" />
                <span>
                  {lang === 'zh' 
                    ? `${sensorData.latitude.toFixed(2)}N, ${sensorData.longitude.toFixed(2)}E`
                    : `${sensorData.latitude.toFixed(2)}°N, ${sensorData.longitude.toFixed(2)}°E`
                  }
                </span>
              </div>
            </div>

            {/* Compass / Bezel Indicator (Simulated) */}
            <div 
              className="absolute top-1.5 w-2.5 h-2.5 rounded-full bg-red-500 ring-2 ring-black"
              style={{ transform: `rotate(${sensorData.heading}deg) translateY(-${(geometry.resolution/2) - 10}px)` }}
            />
          </div>

          {/* Viewport 2: Android Application Drawer (drawerX offset) */}
          <div 
            className="absolute top-0 left-0 w-full h-full flex flex-col px-4 pt-4 text-slate-100 bg-slate-900 transition-transform duration-75 overflow-y-auto scrollbar-none"
            style={{ 
              transform: `translateX(${drawerX}px)`,
              width: `${geometry.resolution}px`,
              height: `${geometry.resolution}px`,
            }}
          >
            {/* Header */}
            <div className="flex items-center gap-1.5 border-b border-slate-800 pb-2 mb-2 shrink-0">
              <Cpu className="w-3.5 h-3.5 text-sky-400" />
              <span className="font-mono text-[10px] uppercase tracking-wider text-slate-400 font-bold">
                {lang === 'zh' ? '应用抽屉' : 'Apps Drawer'}
              </span>
            </div>

            {/* Apps Listing - optimized for 240 vs 320 densities */}
            <div className="flex flex-col gap-1.5 flex-1 pb-4">
              
              {/* App item 1 */}
              <div className="flex items-center justify-between p-2 rounded-lg bg-slate-950/60 border border-slate-800">
                <div className="flex items-center gap-2">
                  <div className="p-1 rounded bg-sky-500/10">
                    <Terminal className="w-3.5 h-3.5 text-sky-400" />
                  </div>
                  <div className="flex flex-col">
                    <span className="text-[10px] font-bold text-slate-300">{t.app1Name}</span>
                    {!isMini && <span className="text-[8px] text-slate-500">libchronoxide FFI v1.3</span>}
                  </div>
                </div>
                <ChevronRight className="w-3 h-3 text-slate-600" />
              </div>

              {/* App item 2 */}
              <div className="flex items-center justify-between p-2 rounded-lg bg-slate-950/60 border border-slate-800">
                <div className="flex items-center gap-2">
                  <div className="p-1 rounded bg-emerald-500/10">
                    <History className="w-3.5 h-3.5 text-emerald-400" />
                  </div>
                  <div className="flex flex-col">
                    <span className="text-[10px] font-bold text-slate-300">{t.app2Name}</span>
                    {!isMini && <span className="text-[8px] text-slate-500">Static allocation locked</span>}
                  </div>
                </div>
                <ChevronRight className="w-3 h-3 text-slate-600" />
              </div>

              {/* App item 3 */}
              <div className="flex items-center justify-between p-2 rounded-lg bg-slate-950/60 border border-slate-800">
                <div className="flex items-center gap-2">
                  <div className="p-1 rounded bg-rose-500/10">
                    <Activity className="w-3.5 h-3.5 text-rose-400" />
                  </div>
                  <div className="flex flex-col">
                    <span className="text-[10px] font-bold text-slate-300">{t.app3Name}</span>
                    {!isMini && <span className="text-[8px] text-slate-500">ALooper Zero-Copy hook</span>}
                  </div>
                </div>
                <ChevronRight className="w-3 h-3 text-slate-600" />
              </div>

              {/* Static RAM alert footer inside the drawer */}
              <div className="mt-auto flex items-center gap-1.5 p-1.5 rounded bg-slate-950 text-slate-500 border border-slate-800/60">
                <CheckCircle2 className="w-3 h-3 text-emerald-500 shrink-0" />
                <span className="text-[8px] font-mono leading-none">MEM OK: 14.2MB Buffer Pool</span>
              </div>

            </div>
          </div>

        </div>

      </div>

      {/* Swipe visual indicator helper */}
      <span className="text-[10px] text-slate-400 mt-2 font-mono flex items-center gap-1">
        <Info className="w-3.5 h-3.5 text-slate-500" />
        {lang === 'zh' ? '在手表盘上按住并向左滑动拉出抽屉' : 'Click and drag left on watch to open drawer'}
      </span>
      
    </div>
  );
}
