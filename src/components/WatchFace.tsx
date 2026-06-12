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
  // let (face_x, drawer_x) = calculate_drawer_viewport(&geo, &touch);
  const maxOffset = -geometry.resolution;
  const clampedOffset = Math.max(maxOffset, Math.min(geometry.resolution, geometry.dragOffsetX));
  
  // App logic: Launcher mode vs Picker mode
  const [systemState, setSystemState] = useState<'Launcher' | 'Picker'>('Launcher');
  const [pickerScrollX, setPickerScrollX] = useState(0);

  // If we are dragging left, the original dial stays at top level and moves left.
  const faceX = systemState === 'Launcher' ? clampedOffset : -geometry.resolution;
  
  // The picker starts at full offset right, and slides in
  const pickerX = systemState === 'Launcher' ? clampedOffset + geometry.resolution : 0;
  
  // Hide the next incoming screen we added previously. Let's adapt it to the picker.
  const nextFaceX = -geometry.resolution;

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
    // Vibration effect simulation handled in real environment
    if (navigator.vibrate) {
      navigator.vibrate(20);
    }
  };

  const activeFacesArray = Array.from({length: 24}, (_, i) => i + 1);

  const handlePointerDown = (clientX: number) => {
    if (!onGeometryChange) return;
    onGeometryChange({
      ...geometry,
      isDragging: true,
      dragStartX: systemState === 'Launcher' ? clientX - geometry.dragOffsetX : clientX - pickerScrollX
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
    } else {
      const targetScroll = -(clientX - geometry.dragStartX); // reverse scroll logic for picker view
      const maxScroll = (24 - 1) * 160;
      setPickerScrollX(Math.max(0, Math.min(targetScroll, maxScroll)));
    }
  }

  const handlePointerUp = () => {
    if (!geometry.isDragging || !onGeometryChange) return;
    
    if (systemState === 'Launcher') {
      const halfRes = geometry.resolution * 0.3; // Threshold for entering picker
      
      if (geometry.dragOffsetX < -halfRes) {
        setSystemState('Picker');
        setPickerScrollX((Math.max(1, activeFaceId) - 1) * 160);
        onGeometryChange({ ...geometry, isDragging: false, dragOffsetX: 0 });
      } else {
        // Return back to center
        onGeometryChange({ ...geometry, isDragging: false, dragOffsetX: 0 });
      }
    } else {
      onGeometryChange({ ...geometry, isDragging: false });
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
            className="absolute top-0 left-0 w-full h-full flex items-center transition-transform duration-75"
            style={{ 
              transform: `translateX(${pickerX}px)`,
              width: `${geometry.resolution}px`,
              height: `${geometry.resolution}px`,
              backgroundColor: '#212124' // 0x2104 hex approximation for dark gray base
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
                if (id === 1) microColor = "bg-blue-600"; // 0x001F
                if (id === 2) microColor = "bg-red-600"; // 0xF800
                if (id === 3) microColor = "bg-green-500"; // 0x07E0
                if (id === 24) microColor = "bg-white"; // 0xFFFF

                return (
                  <div 
                    key={id}
                    onClick={() => onCardClicked(id)}
                    className="relative shrink-0 flex flex-col justify-center items-center shadow-md active:scale-95 transition-transform"
                    style={{
                      width: 120, // card_width
                      height: geometry.resolution - 80, // trimmed top/bottom margins (40..geo.height-40)
                      marginRight: 40, // card_gap
                      backgroundColor: '#1E232E'
                    }}
                  >
                    <div className={`w-8 h-8 rounded-full mb-3 ${microColor}`} />
                    <span className="text-[10px] font-mono text-slate-300 font-bold mb-1">ID: {id}</span>
                    <span className="text-[8px] text-slate-500 leading-tight block text-center px-2">{t.pickerTitle}</span>
                  </div>
                );
              })}
            </div>
            
            {/* Edge mask overlay to simulate physical resolution constraints */}
            <div className="absolute inset-0 pointer-events-none border border-slate-700/30" />
          </div>

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
