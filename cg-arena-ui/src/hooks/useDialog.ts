import { useState } from "react";

export function useDialog<D>(initialData: D): DialogProps<D> {
  const [isOpen, setIsOpen] = useState(false);
  const [data, setData] = useState(initialData);

  const show = (data: D) => {
    setData(data);
    setIsOpen(true);
  };

  const hide = () => {
    setIsOpen(false);
  };

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
