/*
 *  Copyright 2025-2026 Colliery Software
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

import { MantineProvider } from "@mantine/core";
import { QueryClientProvider, QueryClient } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import { AuthProvider } from "./auth/AuthContext";

function renderAt(path: string) {
  return render(
    <MantineProvider>
      <QueryClientProvider client={new QueryClient()}>
        <AuthProvider>
          <MemoryRouter initialEntries={[path]}>
            <App />
          </MemoryRouter>
        </AuthProvider>
      </QueryClientProvider>
    </MantineProvider>,
  );
}

describe("auth gate", () => {
  it("redirects an unauthenticated visit to the connect screen", () => {
    renderAt("/");
    expect(screen.getByText("Connect to Cloacina")).toBeInTheDocument();
  });

  it("shows the connect form fields", () => {
    renderAt("/workflows");
    expect(screen.getByLabelText("Server URL")).toBeInTheDocument();
    expect(screen.getByLabelText("API key")).toBeInTheDocument();
  });
});
