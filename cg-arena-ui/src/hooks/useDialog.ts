import { useCallback, useState } from "react";

export function useDialog<D>(initialData: D): DialogProps<D> {
  const [isOpen, setIsOpen] = useState(false);
  const [data, setData] = useState(initialData);

  const show = useCallback((data: D) => {
    setData(data);
    setIsOpen(true);
  }, [setData, setIsOpen]);

  const hide = useCallback(() => {
    setIsOpen(false);
  }, [setIsOpen]);

  return {
    isOpen,
    data,
    show,
    hide,
  };
}

export interface DialogProps<D> {
  isOpen: boolean;
  data: D;
  show: (data: D) => void;
  hide: () => void;
}
