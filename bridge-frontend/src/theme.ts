// theme.ts

// 1. import `extendTheme` function
import { extendTheme, type ThemeConfig } from "@chakra-ui/react";

const activeLabelStyles = {
    transform: "scale(0.85) translateY(-24px)",
};

// 2. Add your color mode config
const config: ThemeConfig = {
    initialColorMode: "system",
    useSystemColorMode: false,
};

// 3. extend the theme
export const theme = extendTheme({
    ...config,
    components: {
        Form: {
            variants: {
                floating: {
                    container: {
                        _focusWithin: {
                            label: {
                                ...activeLabelStyles,
                            },
                        },
                        "input:not(:placeholder-shown) + label, .chakra-select__wrapper + label, textarea:not(:placeholder-shown) ~ label":
                            {
                                ...activeLabelStyles,
                            },
                        label: {
                            top: 0,
                            left: 0,
                            zIndex: 2,
                            position: "absolute",
                            pointerEvents: "none",
                            mx: 3,
                            px: 1,
                            my: 2,
                            transformOrigin: "left top",
                        },
                    },
                },
            },
        },
    },
});
export default theme;
