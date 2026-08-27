import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, fireEvent, screen } from '@testing-library/svelte';
import DocsHub from '../../lib/docs/DocsHub.svelte';
import GettingStartedDocs from '../../lib/docs/GettingStartedDocs.svelte';
import FrontendDocs from '../../lib/docs/FrontendDocs.svelte';
import BackendDocs from '../../lib/docs/BackendDocs.svelte';
import SystemDocs from '../../lib/docs/SystemDocs.svelte';
import App from '../../App.svelte';
import { authSalt } from '../../lib/stores.js';

describe('Docs Components & Navigation', () => {
  beforeEach(() => {
    authSalt.set('');
    vi.clearAllMocks();
    window.history.pushState({}, '', '/');
  });

  describe('DocsHub.svelte', () => {
    it('renders the documentation hub with 4 distinct options', () => {
      render(DocsHub);

      expect(screen.getByText('Jizifin Documentation')).toBeInTheDocument();
      expect(screen.getByText('Getting Started Guide')).toBeInTheDocument();
      expect(screen.getByText('Frontend Technical Docs')).toBeInTheDocument();
      expect(screen.getByText('Backend Services & DB Engine')).toBeInTheDocument();
      expect(screen.getByText('Architecture & Cryptography')).toBeInTheDocument();

      // Check key action buttons
      expect(screen.getByText('Open Getting Started →')).toBeInTheDocument();
      expect(screen.getByText('Open Frontend Docs →')).toBeInTheDocument();
      expect(screen.getByText('Open Backend Docs →')).toBeInTheDocument();
      expect(screen.getByText('Open Architecture Docs →')).toBeInTheDocument();
    });

    it('filters documentation categories using the search input', async () => {
      render(DocsHub);

      const input = screen.getByPlaceholderText(/Filter documentation topics/i);
      await fireEvent.input(input, { target: { value: 'FastAPI' } });

      expect(screen.getByText('Backend Services & DB Engine')).toBeInTheDocument();
      expect(screen.queryByText('Frontend Technical Docs')).not.toBeInTheDocument();
    });

    it('dispatches navigate event when an option button is clicked', async () => {
      const { component } = render(DocsHub);
      const navigateHandler = vi.fn();
      component.$on('navigate', navigateHandler);

      const gettingStartedBtn = screen.getByText('Open Getting Started →');
      await fireEvent.click(gettingStartedBtn);

      expect(navigateHandler).toHaveBeenCalledTimes(1);
      expect(navigateHandler.mock.calls[0][0].detail).toEqual({ path: '/docs/getting-started' });
    });
  });

  describe('GettingStartedDocs.svelte', () => {
    it('renders 8 step-by-step setup guides', () => {
      render(GettingStartedDocs);

      expect(screen.getByText('Getting Started with Jizifin')).toBeInTheDocument();
      expect(screen.getByText(/Initialize Setup and Set Master Passphrase/i)).toBeInTheDocument();
      expect(screen.getByText(/Add Household Members & Customize Colors/i)).toBeInTheDocument();
      expect(screen.getByText(/Configure Expense Categories & Default Split Ratios/i)).toBeInTheDocument();
      expect(screen.getByText(/Set Up Joint Household Account/i)).toBeInTheDocument();
      expect(screen.getByText(/Log Income Streams and Employment Contracts/i)).toBeInTheDocument();
      expect(screen.getByText(/Define Monthly Budgets & Savings Goals/i)).toBeInTheDocument();
      expect(screen.getByText(/Log Daily Expenses, Split Overrides & Recurring Templates/i)).toBeInTheDocument();
      expect(screen.getByText(/Month-End Settlement and Debt Simplification/i)).toBeInTheDocument();
    });

    it('dispatches navigate event to return to Docs Hub', async () => {
      const { component } = render(GettingStartedDocs);
      const navigateHandler = vi.fn();
      component.$on('navigate', navigateHandler);

      const backBtn = screen.getByText('← Docs Index');
      await fireEvent.click(backBtn);

      expect(navigateHandler).toHaveBeenCalledWith(
        expect.objectContaining({ detail: { path: '/docs' } })
      );
    });
  });

  describe('FrontendDocs.svelte', () => {
    it('renders technical documentation and Settings Validation Matrix', () => {
      render(FrontendDocs);

      expect(screen.getByText('Frontend Architecture & Developer Guide')).toBeInTheDocument();
      expect(screen.getByText(/1\. Stack and Tooling/i)).toBeInTheDocument();
      expect(screen.getByText(/2\. Cryptography/i)).toBeInTheDocument();
      expect(screen.getByText(/3\. Stores and State/i)).toBeInTheDocument();
      expect(screen.getByText(/4\. API Client and WebSockets/i)).toBeInTheDocument();
      expect(screen.getByText(/5\. Component Tree/i)).toBeInTheDocument();
      expect(screen.getByText(/8\. Settings Validation Matrix/i)).toBeInTheDocument();
      expect(screen.getByText(/9\. Testing Suite/i)).toBeInTheDocument();

      // Check validation matrix examples
      expect(screen.getByText(/Split Allocations \(Sum to 100\.0%\)/i)).toBeInTheDocument();
      expect(screen.getByText(/VALID: 60% \/ 40%/i)).toBeInTheDocument();
      expect(screen.getByText(/INVALID: Sum: 90%/i)).toBeInTheDocument();
    });

    it('allows copying code snippets with copy button', async () => {
      const writeTextMock = vi.fn().mockResolvedValue(undefined);
      Object.assign(navigator, {
        clipboard: { writeText: writeTextMock },
      });

      render(FrontendDocs);

      const copyButtons = screen.getAllByRole('button', { name: /Copy/i });
      expect(copyButtons.length).toBeGreaterThan(0);

      await fireEvent.click(copyButtons[0]);
      expect(writeTextMock).toHaveBeenCalled();
    });

    it('dispatches navigate event to return to Docs Hub', async () => {
      const { component } = render(FrontendDocs);
      const navigateHandler = vi.fn();
      component.$on('navigate', navigateHandler);

      const backBtn = screen.getByText('← Docs Index');
      await fireEvent.click(backBtn);

      expect(navigateHandler).toHaveBeenCalledWith(
        expect.objectContaining({ detail: { path: '/docs' } })
      );
    });
  });

  describe('BackendDocs.svelte', () => {
    it('renders exhaustive backend technical documentation and algorithms', () => {
      render(BackendDocs);

      expect(screen.getByText('Backend Services & Database Architecture')).toBeInTheDocument();
      expect(screen.getByText('Backend Reference')).toBeInTheDocument();
      expect(screen.getByText(/1\. Stack and Lifecycle/i)).toBeInTheDocument();
      expect(screen.getByText(/2\. SQLite Schema & WAL Mode/i)).toBeInTheDocument();
      expect(screen.getByText(/3\. Algorithms/i)).toBeInTheDocument();
      expect(screen.getByText(/4\. API Endpoints Catalog/i)).toBeInTheDocument();
      expect(screen.getByText(/5\. WebSockets & Real-Time Events/i)).toBeInTheDocument();
      expect(screen.getByText(/6\. Pytest Test Suite/i)).toBeInTheDocument();

      expect(screen.getByText('Hare-Niemeyer Largest Remainder Distribution')).toBeInTheDocument();
      expect(screen.getByText('Connected Subgraph Debt Simplification')).toBeInTheDocument();
    });

    it('dispatches navigate event to Frontend Docs', async () => {
      const { component } = render(BackendDocs);
      const navigateHandler = vi.fn();
      component.$on('navigate', navigateHandler);

      const readFrontendBtns = screen.getAllByRole('button', { name: /Frontend Docs →/i });
      await fireEvent.click(readFrontendBtns[0]);

      expect(navigateHandler).toHaveBeenCalledWith(
        expect.objectContaining({ detail: { path: '/docs/frontend' } })
      );
    });
  });

  describe('SystemDocs.svelte', () => {
    it('renders cryptographic parameters and system topology', () => {
      render(SystemDocs);

      expect(screen.getAllByText('Architecture & Cryptography').length).toBeGreaterThan(0);
      expect(screen.getByText('Client-Server Boundary')).toBeInTheDocument();
      expect(screen.getByText('Cryptographic Parameters')).toBeInTheDocument();
      expect(screen.getByText('"jizifin-salt-pbkdf2"')).toBeInTheDocument();
      expect(screen.getByText('"jizifin-cryp" (12 bytes)')).toBeInTheDocument();
    });
  });

  describe('App.svelte Docs Routing', () => {
    it('renders DocsHub when URL path is /docs without requiring auth login', () => {
      window.history.pushState({}, '', '/docs');
      render(App);

      expect(screen.getByText('Jizifin Documentation')).toBeInTheDocument();
      expect(screen.getByText('Getting Started Guide')).toBeInTheDocument();
      expect(screen.queryByPlaceholderText('Enter household master passphrase...')).not.toBeInTheDocument();
    });

    it('renders GettingStartedDocs when URL path is /docs/getting-started without requiring auth login', () => {
      window.history.pushState({}, '', '/docs/getting-started');
      render(App);

      expect(screen.getByText('Getting Started with Jizifin')).toBeInTheDocument();
      expect(screen.queryByPlaceholderText('Enter household master passphrase...')).not.toBeInTheDocument();
    });

    it('renders FrontendDocs when URL path is /docs/frontend without requiring auth login', () => {
      window.history.pushState({}, '', '/docs/frontend');
      render(App);

      expect(screen.getByText('Frontend Architecture & Developer Guide')).toBeInTheDocument();
      expect(screen.queryByPlaceholderText('Enter household master passphrase...')).not.toBeInTheDocument();
    });

    it('renders BackendDocs when URL path is /docs/backend', () => {
      window.history.pushState({}, '', '/docs/backend');
      render(App);

      expect(screen.getByText('Backend Services & Database Architecture')).toBeInTheDocument();
    });

    it('renders SystemDocs when URL path is /docs/architecture', () => {
      window.history.pushState({}, '', '/docs/architecture');
      render(App);

      expect(screen.getAllByText('Architecture & Cryptography').length).toBeGreaterThan(0);
    });
  });
});
