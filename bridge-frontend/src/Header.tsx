"use client";

import {
    Box,
    Flex,
    Text,
    IconButton,
    Button,
    useColorModeValue,
    useDisclosure,
    useColorMode,
    Select,
    Menu,
    MenuButton,
    MenuItem,
    MenuList,
} from "@chakra-ui/react";
import { ChevronDownIcon, MoonIcon, SunIcon } from "@chakra-ui/icons";
import { FC } from "react";
import { useConnectWallet, useSetChain } from "@web3-onboard/react";
import configFile from "./config.json";

const config: any = configFile;

export default function WithSubnavigation() {
    const { colorMode, toggleColorMode } = useColorMode();

    const { isOpen, onToggle } = useDisclosure();
    const [{ wallet, connecting }, connect, disconnect] = useConnectWallet();
    const [{ chains, connectedChain, settingChain }, setChain] = useSetChain();

    return (
        <Box>
            <Flex
                minH={"60px"}
                py={{ base: 2 }}
                px={{ base: 4 }}
                borderBottom={0.8}
                borderStyle={"solid"}
                borderColor={useColorModeValue("gray.200", "gray.900")}
                align={"center"}
            >
                <Flex
                    flex={{ base: 1 }}
                    justify={{ base: "center", md: "start" }}
                >
                    <Text fontSize="xl" fontWeight="bold">
                        CarteZcash Bridge \ or LOGO
                    </Text>
                </Flex>
                <IconButton
                    icon={colorMode === "light" ? <MoonIcon /> : <SunIcon />}
                    onClick={toggleColorMode}
                    aria-label={"Toggle Color Mode"}
                />

                {wallet ? (
                    <>
                        <Select
                            width={""}
                            onChange={({ target: { value } }) => {
                                if (config[value] !== undefined) {
                                    setChain({ chainId: value });
                                } else {
                                    alert("No deploy on this chain");
                                }
                            }}
                            value={connectedChain?.id}
                        >
                            {chains.map(({ id, label }) => {
                                return (
                                    <option key={id} value={id}>
                                        {label}
                                    </option>
                                );
                            })}
                        </Select>
                        <Menu closeOnBlur closeOnSelect>
                            <MenuButton
                                as={Button}
                                rightIcon={<ChevronDownIcon />}
                            >
                                {wallet.accounts[0].address.slice(0, 6)}...
                                {wallet.accounts[0].address.slice(-4)}
                            </MenuButton>
                            <MenuList>
                                <MenuItem
                                    onClick={() => {
                                        disconnect(wallet);
                                    }}
                                    maxWidth={"205px"}
                                >
                                    Disconnect
                                </MenuItem>
                            </MenuList>
                        </Menu>
                    </>
                ) : (
                    <Button onClick={() => connect()}>Connect Wallet</Button>
                )}
            </Flex>
        </Box>
    );
}
