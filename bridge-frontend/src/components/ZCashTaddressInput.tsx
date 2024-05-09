import { useState, useMemo } from "react";
import { Input } from "@chakra-ui/react";

export const T_ADDRESS_REGEX = /^t1[a-zA-Z0-9]{1,33}$/;

/**
 * Input for ETH amount with USD conversion.
 *
 * onChange will always be called with the value in ETH
 */
export const ZCashTaddressInput = ({
  value,
  name,
  placeholder,
  onChange,
  disabled,
}: any) => {
  const [transitoryDisplayValue, setTransitoryDisplayValue] = useState<string>();


  // The displayValue is derived from the ether value that is controlled outside of the component
  // In usdMode, it is converted to its usd value, in regular mode it is unaltered
  const displayValue = useMemo(() => {
    const newDisplayValue = value;
    if (transitoryDisplayValue) {
      return transitoryDisplayValue;
    }
    // Clear any transitory display values that might be set
    setTransitoryDisplayValue(undefined);
    return newDisplayValue;
  }, [transitoryDisplayValue, value]);

  const handleChangeNumber = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    if (newValue && !T_ADDRESS_REGEX.test(newValue)) {
      return;
    }

    onChange(newValue);
  };

  return (
    <Input
      name={name}
      value={displayValue}
      placeholder={placeholder}
      onChange={handleChangeNumber}
      disabled={disabled}
    />
  );
};
