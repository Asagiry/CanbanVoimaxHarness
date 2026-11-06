import { useCallback, useState } from 'react';
import { toProviderIdValue, type Provider } from '../../../stores/settings/providers';

export function useProviderEditorState() {
  const [editingProvider, setEditingProvider] = useState<Provider | null>(null);

  const openEditor = useCallback((provider: Provider) => {
    setEditingProvider(provider);
  }, []);

  const openNewProvider = useCallback((providers: Provider[]) => {
    const id = toProviderIdValue(`custom-${providers.length + 1}`);
    setEditingProvider({
      id,
      name: 'custom',
      isCustom: true,
      apiKey: '',
      baseUrl: '',
      protocol: 'openai',
      models: [],
    });
  }, []);

  const closeEditor = useCallback(() => {
    setEditingProvider(null);
  }, []);

  return { editingProvider, openEditor, openNewProvider, closeEditor };
}
