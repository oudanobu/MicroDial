export interface WeatherInfo {
  temperature: number; // -128 to 127
  conditionCode: number;
  locationId: number;
  humidity: number; // 0-100
}

export interface CompactDate {
  year: number; // 1900 - 2200
  month: number; // 1 - 12
  day: number; // 1 - 31
}

export interface SensorData {
  steps: number;
  heading: number;
  longitude: number;
  latitude: number;
  pressure: number;
  weather: WeatherInfo | null;
}
