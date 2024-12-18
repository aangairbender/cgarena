import { JdenticonConfig, toSvg } from "jdenticon";
import React from "react";

interface IdenticonProps {
  input: string;
  size: number;
}

const config: JdenticonConfig = {
    backColor: "#fff",
    padding: 0,
};

const Identicon: React.FC<IdenticonProps> = ({ input, size }) => {
  const svgString = toSvg(input, size, config);

  return <div dangerouslySetInnerHTML={{ __html: svgString }}></div>;
};

export default Identicon;
