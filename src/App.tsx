/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { useState } from 'react';
import { WatchFace } from './components/WatchFace';
import { SensorControls } from './components/SensorControls';
import { GeometryControls, GeometrySettings } from './components/GeometryControls';
import { SensorData } from './types';
import { Language, translations } from './lib/translations';
import { Globe, Cpu, CheckCircle } from 'lucide-react';

export default function App() {
  const [lang, setLang] = useState<Language>('zh');

  // Initialize with the exact mock provider data from the Rust unit test
  const [sensorData, setSensorData] = useState<SensorData>({
    steps: 8432,
    heading: 180.0,
    longitude: 116.4074,
    latitude: 39.9042, // Beijing Tiananmen coords
    pressure: 1011.5,
    weather: {
      temperature: 24,
      conditionCode: 1,
      locationId: 110000,
      humidity: 50
    }
  });

  // Physical screen parameters supporting round/square and low resolution
  const [geometry, setGeometry] = useState<GeometrySettings>({
    shape: 'Round',
    resolution: 320,
    dragStartX: 0,
    dragOffsetX: 0,
    isDragging: false,
    densityScale: 1.0
  });

  const t = translations[lang];

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col justify-between font-sans">
      
      {/* Top Navigation / Header */}
      <header className="border-b border-slate-200 bg-white shadow-sm/50 px-6 py-4">
        <div className="max-w-7xl mx-auto flex justify-between items-center">
          <div className="flex items-center gap-3">
            <Cpu className="w-6 h-6 text-slate-800 animate-pulse" />
            <span className="font-mono font-bold tracking-tight text-slate-900 text-lg">
              ChronOxide <span className="text-[10px] text-sky-600 bg-sky-50 px-2 py-0.5 rounded border border-sky-100 font-bold ml-1">Rust Low-Memory Kernel v1.3.0</span>
            </span>
          </div>

          <button
            onClick={() => setLang(lang === 'zh' ? 'en' : 'zh')}
            className="flex items-center gap-2 text-xs font-semibold bg-slate-100 hover:bg-slate-200 text-slate-700 px-3.5 py-1.5 rounded-lg transition"
          >
            <Globe className="w-3.5 h-3.5" />
            <span>{t.toggleLanguage}</span>
          </button>
        </div>
      </header>

      {/* Main Content Area */}
      <main className="flex-1 max-w-7xl w-full mx-auto p-6 md:p-8 flex flex-col items-center justify-center gap-8 lg:gap-12">
        <div className="text-center max-w-2xl">
          <h1 className="text-3xl md:text-4xl font-extrabold text-slate-900 tracking-tight leading-tight">
            {t.brandName}
          </h1>
          <p className="text-sm md:text-base text-slate-500 mt-3 max-w-xl mx-auto">
            {t.subtitle}
          </p>
          <div className="mt-4 flex flex-wrap justify-center gap-3">
            <span className="text-xs font-mono font-medium text-emerald-700 bg-emerald-50 border border-emerald-200 px-2.5 py-1 rounded-md flex items-center gap-1 shadow-sm">
              <CheckCircle className="w-3.5 h-3.5" />
              {t.memoryMode}
            </span>
            <span className="text-xs font-mono font-medium text-slate-700 bg-slate-100 border border-slate-200 px-2.5 py-1 rounded-md shadow-sm">
              {t.compileStatus}
            </span>
          </div>
        </div>

        {/* 3-Column Responsive Dashboard */}
        <div className="w-full flex flex-col xl:flex-row items-start justify-center gap-8 xl:gap-8">
          
          {/* Column 1: Hardware Sensors */}
          <div className="w-full xl:w-auto flex justify-center shrink-0">
            <SensorControls data={sensorData} onChange={setSensorData} lang={lang} />
          </div>

          {/* Column 2: Rust Compiled Geometry Bounds */}
          <div className="w-full xl:w-auto flex justify-center shrink-0">
            <GeometryControls settings={geometry} onChange={setGeometry} lang={lang} />
          </div>

          {/* Column 3: Canvas Emulator Stage */}
          <div className="flex flex-col items-center flex-1 min-w-[340px] justify-center pt-2">
            <div className="bg-slate-200 text-slate-600 text-[10px] font-mono font-bold uppercase tracking-wider px-3.5 py-1.5 rounded-md mb-6 border border-slate-300 shadow-inner">
              {t.displaySurface} ({geometry.resolution}×{geometry.resolution} px)
            </div>
            
            <WatchFace 
              sensorData={sensorData} 
              lang={lang} 
              geometry={geometry} 
              onGeometryChange={setGeometry}
            />
            
            <div className="mt-8 text-center bg-white/40 border border-slate-200/50 rounded-2xl px-6 py-4 max-w-xs backdrop-blur-sm">
              <h2 className="text-sm font-bold text-slate-800 tracking-tight">
                {t.watchfaceName}
              </h2>
              <p className="text-[11px] text-slate-400 mt-1 lines-2 leading-relaxed">
                {t.uncompressedWarning}
              </p>
            </div>
          </div>

        </div>
      </main>

      {/* Embedded Watch Architecture Status Footer */}
      <footer className="bg-white border-t border-slate-200 py-4 px-6 text-center text-[11px] font-mono text-slate-400">
        ChronOxide Co., Ltd. &bull; {lang === 'zh' ? '在 512MB RAM 下实现 15MB 极限内存开销守护' : 'Safeguarding 15MB RAM Overhead on 512MB Systems'}
      </footer>

    </div>
  );
}
