import { useCallback, useState } from "react";

export function useDialog<D>(): DialogProps<D> {
  const [isOpen, setIsOpen] = useState(false);
  const [data, setData] = useState<D | undefined>();

  const show = useCallback((data: D) => {
    setData(data);
    setIsOpen(true);
  }, [setData, setIsOpen]);

  const hide = useCallback(() => {
    setIsOpen(false);
    setData(undefined);
  }, [setData, setIsOpen]);

  return {
    isOpen,
    data,
    show,
    hide,
  };
}

export interface DialogProps<D> {
  isOpen: boolean;
  data: D | undefined;
  show: (data: D) => void;
  hide: () => void;
}
