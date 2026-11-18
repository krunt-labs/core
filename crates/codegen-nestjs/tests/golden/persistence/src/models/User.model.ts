// krunt:source=node:40
export interface User {
  email?: string;
  id: number;
  profile?: {
      bio?: string;
      displayName: string;
    };
  tags: Array<string>;
}

export function validateUser(value: unknown): User {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('User must be an object.');
  }
  const record = value as Record<string, unknown>;
  if (Object.prototype.hasOwnProperty.call(record, 'email') && (typeof record.email !== 'string')) {
    throw new Error('User.email must be a string.');
  }
  if (!Object.prototype.hasOwnProperty.call(record, 'id') || typeof record.id !== 'number' || !Number.isInteger(record.id)) {
    throw new Error('User.id must be a integer.');
  }
  if (Object.prototype.hasOwnProperty.call(record, 'profile') && (typeof record.profile !== 'object' || record.profile === null || Array.isArray(record.profile) || Object.prototype.hasOwnProperty.call(record.profile, 'bio') && (typeof (record.profile as Record<string, unknown>).bio !== 'string') || !Object.prototype.hasOwnProperty.call(record.profile, 'displayName') || typeof (record.profile as Record<string, unknown>).displayName !== 'string')) {
    throw new Error('User.profile must be a object.');
  }
  if (!Object.prototype.hasOwnProperty.call(record, 'tags') || !Array.isArray(record.tags) || !record.tags.every((item) => !(typeof item !== 'string'))) {
    throw new Error('User.tags must be a array.');
  }
  return {
    email: record.email as string | undefined,
    id: record.id as number,
    profile: record.profile as {
      bio?: string;
      displayName: string;
    } | undefined,
    tags: record.tags as Array<string>,
  };
}
