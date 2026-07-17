import type { Profile } from "./types";

type EnergyInput = Pick<Profile, "birthDate" | "sexForEnergy" | "activityLevel" | "heightCm" | "weightKg">;

const coefficients = {
  male: {
    inactive: [753.07, -10.83, 6.5, 14.1],
    low_active: [581.47, -10.83, 8.3, 14.94],
    active: [1004.82, -10.83, 6.52, 15.91],
    very_active: [-517.88, -10.83, 15.61, 19.11],
  },
  female: {
    inactive: [584.9, -7.01, 5.72, 11.71],
    low_active: [575.77, -7.01, 6.6, 12.14],
    active: [710.25, -7.01, 6.54, 12.34],
    very_active: [511.83, -7.01, 9.07, 12.56],
  },
} as const;

export const activityLabels: Record<Profile["activityLevel"], { title: string; description: string }> = {
  inactive: { title: "Überwiegend sitzend", description: "Wenig Bewegung im Alltag, meist sitzende Tätigkeiten." },
  low_active: { title: "Leicht aktiv", description: "Regelmäßige Wege zu Fuß und etwas Bewegung im Alltag." },
  active: { title: "Aktiv", description: "Viel Bewegung im Alltag oder körperlich aktive Arbeit." },
  very_active: { title: "Sehr aktiv", description: "Sehr bewegter Alltag oder überwiegend schwere körperliche Arbeit." },
};

export function ageOnDate(birthDate?: string, today = new Date()): number | null {
  if (!birthDate) return null;
  const match = /^(\d{4})-(\d{2})-(\d{2})$/.exec(birthDate);
  if (!match) return null;
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const birth = new Date(year, month - 1, day);
  if (birth.getFullYear() !== year || birth.getMonth() !== month - 1 || birth.getDate() !== day || birth > today) return null;
  let age = today.getFullYear() - year;
  if (today.getMonth() < month - 1 || (today.getMonth() === month - 1 && today.getDate() < day)) age -= 1;
  return age;
}

export function calculateEnergyTarget(input: EnergyInput, today = new Date()): { calories: number; age: number } | null {
  const age = ageOnDate(input.birthDate, today);
  if (age === null || age < 19 || !input.sexForEnergy || !input.heightCm || !input.weightKg) return null;
  if (input.heightCm <= 0 || input.weightKg <= 0) return null;
  const [base, ageFactor, heightFactor, weightFactor] = coefficients[input.sexForEnergy][input.activityLevel];
  return { calories: Math.round(base + ageFactor * age + heightFactor * input.heightCm + weightFactor * input.weightKg), age };
}
