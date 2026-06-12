export function calculateAltitude(pressure: number): number {
  // Estimated altitude calculation based on standard sea-level pressure
  // H = 44330 * (1 - (P/P_0)^(1/5.255))
  return 44330.0 * (1.0 - Math.pow(pressure / 1013.25, 1.0 / 5.255));
}

export function validateCalendarBounds(year: number, month: number, day: number): boolean {
  if (year < 1900 || year > 2200) {
    return false;
  }
  if (month < 1 || month > 12 || day < 1 || day > 31) {
    return false;
  }
  return true;
}
