import { useEffect, useState } from 'react';
import { calculateAltitude } from '../lib/calc';
import { SensorData } from '../types';
import { Language, translations } from '../lib/translations';
import { Compass, Footprints, MapPin, Gauge, CloudRain, Mountain } from 'lucide-react';

interface WatchFaceProps {
  sensorData: SensorData;
  lang: Language;
}

export function WatchFace({ sensorData, lang }: WatchFaceProps) {
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

  return (
    <div className="relative w-80 h-80 rounded-full bg-slate-950 border-[12px] border-slate-800 shadow-2xl flex flex-col items-center justify-center text-slate-100 overflow-hidden ring-4 ring-slate-900">
      {/* Time Display */}
      <div className="mt-4 flex flex-col items-center">
        <span className="text-5xl font-mono font-medium tracking-tighter">
          {time.getHours().toString().padStart(2, '0')}:
          {time.getMinutes().toString().padStart(2, '0')}
        </span>
        <span className="text-xs font-sans text-slate-400 font-medium uppercase tracking-wider mt-1.5">
          {dateStr}
        </span>
      </div>

      {/* Primary Metrics */}
      <div className="flex gap-8 mt-6">
        <div className="flex flex-col items-center justify-center">
          <Footprints className="w-5 h-5 text-emerald-400 mb-1" />
          <span className="text-lg font-mono font-medium">{sensorData.steps}</span>
          <span className="text-[10px] text-slate-500">{t.steps}</span>
        </div>
        <div className="flex flex-col items-center justify-center">
          <Mountain className="w-5 h-5 text-sky-400 mb-1" />
          <span className="text-lg font-mono font-medium">
            {formattedAltitude}{lang === 'zh' ? '米' : 'm'}
          </span>
          <span className="text-[10px] text-slate-500">{lang === 'zh' ? '海拔' : 'Alt'}</span>
        </div>
      </div>

      {/* Secondary Metrics / Bottom Arc */}
      <div className="absolute bottom-8 flex flex-col items-center w-full px-8 text-xs font-mono text-slate-400">
        <div className="flex justify-between w-full mb-1">
          <div className="flex items-center gap-1">
            <Gauge className="w-3 h-3 text-slate-500" />
            <span>{sensorData.pressure.toFixed(1)} hPa</span>
          </div>
          <div className="flex items-center gap-1">
            <CloudRain className="w-3 h-3 text-slate-500" />
            {sensorData.weather ? (
              <span>{sensorData.weather.temperature}°C / {sensorData.weather.humidity}%</span>
            ) : (
              <span>--</span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1 text-[11px] justify-center text-slate-500">
          <MapPin className="w-3 h-3 text-slate-500" />
          <span>
            {lang === 'zh' 
              ? `北纬 ${sensorData.latitude.toFixed(4)}°, 东经 ${sensorData.longitude.toFixed(4)}°`
              : `${sensorData.latitude.toFixed(4)}°N, ${sensorData.longitude.toFixed(4)}°E`
            }
          </span>
        </div>
      </div>

      {/* Compass / Bezel Indicator (Simulated) */}
      <div 
        className="absolute top-2 w-2 h-2 rounded-full bg-red-500"
        style={{ transform: `rotate(${sensorData.heading}deg) translateY(-140px)` }}
      />
    </div>
  );
}
