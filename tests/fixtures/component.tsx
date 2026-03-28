import React from 'react';

interface AuthProps {
  token: string;
  onLogout: () => void;
}

export function AuthButton({ token, onLogout }: AuthProps) {
  if (!token) {
    return <button>Login</button>;
  }
  return <button onClick={onLogout}>Logout</button>;
}
