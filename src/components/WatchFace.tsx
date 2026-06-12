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
  activeFaceId?: number;
  onFaceChange?: (id: number) => void;
}

export function WatchFace({ sensorData, lang, geometry, onGeometryChange, activeFaceId = 0, onFaceChange }: WatchFaceProps) {
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
  const maxOffset = -geometry.resolution;
  const clampedOffset = Math.max(maxOffset, Math.min(geometry.resolution, geometry.dragOffsetX));
  
  // App logic: Launcher mode vs Picker mode (Right Swipe) vs AppDrawer mode (Left Swipe)
  const [systemState, setSystemState] = useState<'Launcher' | 'Picker' | 'AppDrawer'>('Launcher');
  const [pickerScrollX, setPickerScrollX] = useState(0);
  const [toastMessage, setToastMessage] = useState<string | null>(null);

  const showToast = (msg: string) => {
    setToastMessage(msg);
    setTimeout(() => {
      setToastMessage(null);
    }, 2500);
  };

  // Three-panel physical slide layout matching smartwatch conventions:
  let faceX: number = 0;
  let pickerX: number = -geometry.resolution;
  let drawerX: number = geometry.resolution;

  if (systemState === 'Launcher') {
    faceX = clampedOffset;
    pickerX = clampedOffset - geometry.resolution; // Picker comes in from left on Right Swipe (offset > 0)
    drawerX = clampedOffset + geometry.resolution; // AppDrawer comes in from right on Left Swipe (offset < 0)
  } else if (systemState === 'Picker') {
    // Picker is centered. Swiping right slides it out to reveal dial underneath
    const slideOffset = Math.max(0, geometry.dragOffsetX); 
    pickerX = slideOffset;
    faceX = slideOffset + geometry.resolution;
    drawerX = slideOffset + geometry.resolution * 2;
  } else if (systemState === 'AppDrawer') {
    // AppDrawer is centered. Swiping left slides it out to reveal dial underneath
    const slideOffset = Math.max(0, geometry.dragOffsetX); // slide right to exit
    drawerX = slideOffset;
    faceX = slideOffset - geometry.resolution;
    pickerX = slideOffset - geometry.resolution * 2;
  }

  // Determine watch face theme coloring
  let bgClass = "bg-slate-950"; // default ID 0
  let primaryTextClass = "text-white";
  let accentClass1 = "text-emerald-400";
  let accentClass2 = "text-sky-400";
  let faceLabel = t.face0;

  if (activeFaceId === 1) {
    bgClass = "bg-red-950/80";
    accentClass1 = "text-rose-400";
    accentClass2 = "text-amber-400";
    faceLabel = t.face1;
  } else if (activeFaceId === 2) {
    bgClass = "bg-emerald-950/80";
    accentClass1 = "text-teal-400";
    accentClass2 = "text-lime-400";
    faceLabel = t.face2;
  } else if (activeFaceId === 3) {
    bgClass = "bg-sky-950/80";
    accentClass1 = "text-cyan-400";
    accentClass2 = "text-blue-400";
    faceLabel = t.face3;
  } else if (activeFaceId === 23) {
    bgClass = "bg-purple-950/80";
    accentClass1 = "text-fuchsia-400";
    accentClass2 = "text-pink-400";
    faceLabel = t.face23;
  } else if (activeFaceId === 24) {
    bgClass = "bg-black";
    primaryTextClass = "text-slate-300";
    accentClass1 = "text-slate-500";
    accentClass2 = "text-slate-500";
    faceLabel = t.face24;
  } else if (activeFaceId > 3 && activeFaceId < 23) {
    bgClass = "bg-slate-900";
    faceLabel = `Custom ID: ${activeFaceId}`;
  }

  // Render scale for low-res (240x240 fits neatly in a 320px physical container container)
  const isMini = geometry.resolution === 240;

  // Handle the logic of Card clicked
  const onCardClicked = (id: number) => {
    if (onFaceChange) {
      onFaceChange(id);
    }
    setSystemState('Launcher');
    showToast(lang === 'zh' ? `已激活 Rust 驱动 ${id} 号` : `Loaded Watchface Driver ${id}`);
    if (navigator.vibrate) {
      navigator.vibrate(20);
    }
  };

  const activeFacesArray = Array.from({length: 24}, (_, i) => i + 1);

  const handlePointerDown = (clientX: number) => {
    if (!onGeometryChange) return;
    let dragStartCalculated = clientX;
    if (systemState === 'Launcher') {
      dragStartCalculated = clientX - geometry.dragOffsetX;
    } else if (systemState === 'Picker') {
      dragStartCalculated = clientX - pickerScrollX;
    } else if (systemState === 'AppDrawer') {
      dragStartCalculated = clientX - geometry.dragOffsetX;
    }
    onGeometryChange({
      ...geometry,
      isDragging: true,
      dragStartX: dragStartCalculated
    });
  }

  const handlePointerMove = (clientX: number) => {
    if (!geometry.isDragging || !onGeometryChange) return;
    if (systemState === 'Launcher') {
      const offset = clientX - geometry.dragStartX;
      const clamped = Math.max(-geometry.resolution, Math.min(geometry.resolution, offset));
      onGeometryChange({
        ...geometry,
        dragOffsetX: clamped
      });
    } else if (systemState === 'Picker') {
      // In Picker, dragging left/right scrolls the selector. If we exceed right scroll threshold, we exit.
      const targetScroll = -(clientX - geometry.dragStartX);
      const maxScroll = (24 - 1) * 160;
      if (clientX - geometry.dragStartX > 150) {
        // swipe right far enough to trigger back to launcher
        onGeometryChange({
          ...geometry,
          dragOffsetX: clientX - geometry.dragStartX
        });
      } else {
        setPickerScrollX(Math.max(0, Math.min(targetScroll, maxScroll)));
      }
    } else if (systemState === 'AppDrawer') {
      // In App Drawer, swipe right exits back to Launcher
      const offset = clientX - geometry.dragStartX;
      const clamped = Math.max(0, Math.min(geometry.resolution, offset));
      onGeometryChange({
        ...geometry,
        dragOffsetX: clamped
      });
    }
  }

  const handlePointerUp = () => {
    if (!geometry.isDragging || !onGeometryChange) return;
    
    if (systemState === 'Launcher') {
      const threshold = geometry.resolution * 0.25; // 25% threshold
      
      if (geometry.dragOffsetX > threshold) {
        // Swiped right -> enters Watch Face Picker
        setSystemState('Picker');
        setPickerScrollX((Math.max(1, activeFaceId) - 1) * 160);
      } else if (geometry.dragOffsetX < -threshold) {
        // Swiped left -> enters App Drawer
        setSystemState('AppDrawer');
      }
      onGeometryChange({ ...geometry, isDragging: false, dragOffsetX: 0 });
    } else if (systemState === 'Picker') {
      if (geometry.dragOffsetX > geometry.resolution * 0.3) {
        setSystemState('Launcher');
      }
      onGeometryChange({ ...geometry, isDragging: false, dragOffsetX: 0 });
    } else if (systemState === 'AppDrawer') {
      if (geometry.dragOffsetX > geometry.resolution * 0.3) {
        setSystemState('Launcher');
      }
      onGeometryChange({ ...geometry, isDragging: false, dragOffsetX: 0 });
    }
  };

  const handleMouseDown = (e: React.MouseEvent) => handlePointerDown(e.clientX);
  const handleMouseMove = (e: React.MouseEvent) => handlePointerMove(e.clientX);
  const handleMouseUpOrLeave = () => handlePointerUp();
  const handleTouchStart = (e: React.TouchEvent) => handlePointerDown(e.touches[0].clientX);
  const handleTouchMove = (e: React.TouchEvent) => handlePointerMove(e.touches[0].clientX);

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
            className={`absolute top-0 left-0 w-full h-full flex flex-col items-center justify-center transition-transform duration-75 ${bgClass} ${primaryTextClass}`}
            style={{ 
              transform: `translateX(${faceX}px)`,
              width: `${geometry.resolution}px`,
              height: `${geometry.resolution}px`,
            }}
          >
            {/* Singleton Current Face Indicator */}
            <div className="absolute top-2 w-full flex justify-center opacity-40">
              <span className={`text-[8px] font-mono px-1.5 py-0.5 rounded border border-white/20 bg-black/20 text-white`}>
                {faceLabel}
              </span>
            </div>

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
                <Footprints className={`${isMini ? 'w-4 h-4' : 'w-5 h-5'} ${accentClass1} mb-0.5`} />
                <span className={`font-mono font-medium ${isMini ? 'text-sm' : 'text-lg'}`}>{sensorData.steps}</span>
                <span className="text-[10px] opacity-70 font-medium">{t.steps}</span>
              </div>
              <div className="flex flex-col items-center justify-center">
                <Mountain className={`${isMini ? 'w-4 h-4' : 'w-5 h-5'} ${accentClass2} mb-0.5`} />
                <span className={`font-mono font-medium ${isMini ? 'text-sm' : 'text-lg'}`}>
                  {formattedAltitude}{lang === 'zh' ? '米' : 'm'}
                </span>
                <span className="text-[10px] opacity-70 font-medium">{lang === 'zh' ? '海拔' : 'Alt'}</span>
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

          {/* Viewport 2: Watch Face Picker (pickerX offset) */}
          <div 
            className="absolute top-0 left-0 w-full h-full flex items-center transition-transform duration-75 select-none"
            style={{ 
              transform: `translateX(${pickerX}px)`,
              width: `${geometry.resolution}px`,
              height: `${geometry.resolution}px`,
              backgroundColor: '#1E232E' // 0x2104 hex approximation for dark gray base
            }}
          >
            {/* The internal scroll container for the 1-24 cards */}
            <div 
              className="absolute top-0 bottom-0 flex items-center"
              style={{ transform: `translateX(${-pickerScrollX + 40}px)` }}
            >
              {activeFacesArray.map((id) => {
                // Dynamically build card appearance based on rust specs
                let microColor = "bg-slate-600";
                if (id === 1) microColor = "bg-blue-600 animate-pulse"; // 0x001F
                if (id === 2) microColor = "bg-red-600 shadow-inner"; // 0xF800
                if (id === 3) microColor = "bg-emerald-500 shadow-lg"; // 0x07E0
                if (id === 24) microColor = "bg-white shadow"; // 0xFFFF

                return (
                  <div 
                    key={id}
                    onClick={() => onCardClicked(id)}
                    className={`relative shrink-0 flex flex-col justify-center items-center rounded-2xl active:scale-95 transition-all cursor-pointer ${
                      activeFaceId === id ? "border-2 border-emerald-500 ring-2 ring-emerald-500/20" : "border border-slate-700/50"
                    }`}
                    style={{
                      width: 120, // card_width
                      height: geometry.resolution - 80, // trimmed top/bottom margins (40..geo.height-40)
                      marginRight: 40, // card_gap
                      backgroundColor: '#11141C'
                    }}
                  >
                    <div className={`w-8 h-8 rounded-full mb-3 flex items-center justify-center text-[10px] font-bold text-white shadow-md ${microColor}`} />
                    <span className="text-[10px] font-mono text-slate-300 font-bold mb-1">ID: {id}</span>
                    <span className="text-[8px] text-slate-500 leading-tight block text-center px-2">{t.pickerTitle}</span>
                  </div>
                );
              })}
            </div>
            
            {/* Edge mask overlay to simulate physical resolution constraints */}
            <div className="absolute inset-0 pointer-events-none border border-slate-700/30" />
          </div>

          {/* Viewport 3: App Drawer / Settings (drawerX offset) */}
          <div 
            className="absolute top-0 left-0 w-full h-full flex flex-col items-center justify-start py-6 transition-transform duration-75 text-white select-none"
            style={{ 
              transform: `translateX(${drawerX}px)`,
              width: `${geometry.resolution}px`,
              height: `${geometry.resolution}px`,
              backgroundColor: '#0A0D14' // Deep sleek dark background
            }}
          >
            {/* Rounded Bezel Top Header */}
            <div className={`text-center font-bold tracking-tight text-slate-300 uppercase font-mono border-b border-slate-800/80 pb-2 w-full px-4 mb-2 flex items-center justify-center gap-1.5 ${isMini ? 'text-[9px]' : 'text-xs'}`}>
              <Cpu className="w-3.5 h-3.5 text-amber-500 animate-pulse" />
              <span>{t.activeDrawer}</span>
            </div>

            {/* Vertically scrollable list of apps */}
            <div className={`flex-1 w-full overflow-y-auto px-3.5 flex flex-col gap-1.5 scrollbar-thin scrollbar-thumb-slate-800 pb-6 ${isMini ? "pt-1" : "pt-2"}`}>
              
              {/* App 1: Terminal */}
              <div 
                onClick={() => {
                  showToast(lang === 'zh' ? '正在加载：系统终端内核...' : 'Loading: Systems Terminal Core...');
                  setSystemState('Launcher');
                }}
                className={`flex items-center gap-2.5 bg-slate-900/75 hover:bg-slate-800 border border-slate-800/80 rounded-xl cursor-pointer active:scale-95 transition ${
                  isMini ? 'p-1.5' : 'p-2'
                }`}
              >
                <div className="w-7 h-7 rounded-lg bg-amber-500/10 border border-amber-500/20 flex items-center justify-center text-amber-400 shrink-0">
                  <Terminal className="w-3.5 h-3.5" />
                </div>
                <div className="flex-1 min-w-0 text-left">
                  <div className={`font-mono font-bold text-slate-200 truncate ${isMini ? 'text-[9px]' : 'text-[11px]'}`}>{t.app1Name}</div>
                  <div className="text-[8px] text-slate-500 truncate">TTY /dev/ttyS0 live</div>
                </div>
                <ChevronRight className="w-3 h-3 text-slate-600 shrink-0" />
              </div>

              {/* App 2: Static Memory */}
              <div 
                onClick={() => {
                  showToast(lang === 'zh' ? '内存常驻机制评估：0 耗损 100% 正常' : 'Static pool evaluate: 0 leak, 100% stable');
                  setSystemState('Launcher');
                }}
                className={`flex items-center gap-2.5 bg-slate-900/75 hover:bg-slate-800 border border-slate-800/80 rounded-xl cursor-pointer active:scale-95 transition ${
                  isMini ? 'p-1.5' : 'p-2'
                }`}
              >
                <div className="w-7 h-7 rounded-lg bg-sky-500/10 border border-sky-500/20 flex items-center justify-center text-sky-400 shrink-0">
                  <History className="w-3.5 h-3.5" />
                </div>
                <div className="flex-1 min-w-0 text-left">
                  <div className={`font-mono font-bold text-slate-200 truncate ${isMini ? 'text-[9px]' : 'text-[11px]'}`}>{t.app2Name}</div>
                  <div className="text-[8px] text-slate-500 truncate">Slab Allocation Guard</div>
                </div>
                <ChevronRight className="w-3 h-3 text-slate-600 shrink-0" />
              </div>

              {/* App 3: Sensory Zero-Copy Looper */}
              <div 
                onClick={() => {
                  showToast(lang === 'zh' ? '零拷贝 ALooper 管道校准脉冲成功' : 'ALooper sensory pipeline calibrated');
                  setSystemState('Launcher');
                }}
                className={`flex items-center gap-2.5 bg-slate-900/75 hover:bg-slate-800 border border-slate-800/80 rounded-xl cursor-pointer active:scale-95 transition ${
                  isMini ? 'p-1.5' : 'p-2'
                }`}
              >
                <div className="w-7 h-7 rounded-lg bg-emerald-500/10 border border-emerald-500/20 flex items-center justify-center text-emerald-400 shrink-0">
                  <Activity className="w-3.5 h-3.5" />
                </div>
                <div className="flex-1 min-w-0 text-left">
                  <div className={`font-mono font-bold text-slate-200 truncate ${isMini ? 'text-[9px]' : 'text-[11px]'}`}>{t.app3Name}</div>
                  <div className="text-[8px] text-slate-500 truncate">Direct JNI Sensor Pipe</div>
                </div>
                <ChevronRight className="w-3 h-3 text-slate-600 shrink-0" />
              </div>

            </div>
          </div>

          {/* Premium In-Watch Micro Toast Notification Overlay */}
          {toastMessage && (
            <div className="absolute bottom-16 left-1/2 -translate-x-1/2 bg-slate-900/95 border border-slate-800 shadow-2xl px-3 py-1.5 rounded-full text-center max-w-[85%] z-50 flex items-center gap-1.5 animate-in fade-in slide-in-from-bottom-4 duration-300">
              <CheckCircle2 className="w-3 h-3 text-emerald-400" />
              <span className="text-[9px] text-slate-200 font-mono font-bold leading-tight">{toastMessage}</span>
            </div>
          )}

        </div>

      </div>

      {/* Swipe visual indicator helper */}
      <div className="text-[10px] text-slate-400 mt-2 font-mono flex flex-col items-center gap-1">
        <span className="flex items-center gap-1">
          <Info className="w-3.5 h-3.5 text-slate-500" />
          {systemState === 'Launcher' ? t.switchWatchface : 'Swipe or click to select face'}
        </span>
        <span className="text-[9px] text-emerald-600 bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200">
          {t.activeWatchfaceBadge}: {activeFaceId}
        </span>
      </div>
      
    </div>
  );
}
