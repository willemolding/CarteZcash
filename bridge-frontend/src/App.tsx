// Copyright 2022 Cartesi Pte. Ltd.

// Licensed under the Apache License, Version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy
// of the license at http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.

import { FC } from "react";
import injectedModule from "@web3-onboard/injected-wallets";
import { init, useConnectWallet } from "@web3-onboard/react";
import { useState } from "react";

import { GraphQLProvider } from "./GraphQL";
import { Transfers } from "./Transfers";
import { Network } from "./Network";
import configFile from "./config.json";
import "./App.css";
import {
    Input,
    Box,
    InputGroup,
    InputLeftAddon,
    Stack,
    SimpleGrid,
    useColorMode,
    Button,
    Heading,
    Text,
    Image,
    extendTheme,
} from "@chakra-ui/react";
import banner from "./banner.png";
import Header from "./Header";

const config: any = configFile;

const injected: any = injectedModule();
init({
    wallets: [injected],
    chains: Object.entries(config).map(([k, v]: [string, any], i) => ({
        id: k,
        token: v.token,
        label: v.label,
        rpcUrl: v.rpcUrl,
    })),
    appMetadata: {
        name: "Cartesi Rollups Test DApp",
        icon: "<svg><svg/>",
        description: "Demo app for Cartesi Rollups",
        recommendedInjectedWallets: [
            { name: "MetaMask", url: "https://metamask.io" },
        ],
    },
});

const App: FC = () => {
    const [dappAddress, setDappAddress] = useState<string>(
        "0x70ac08179605AF2D9e75782b8DEcDD3c22aA4D0C"
    );

    const [{ wallet, connecting }, connect] = useConnectWallet();

    return (
        <>
            <Header dappAddress={dappAddress} setDappAddress={setDappAddress} />
            <SimpleGrid columns={1} marginX={"25%"} alignContent={"center"}>
                {!wallet && (
                    <Box mt="20" alignContent="center">
                        <Stack>
                            <Heading>CarteZcash Bridge</Heading>
                            <Text>
                                Connect a wallet to deposit or withdraw Eth from
                                the rollup
                            </Text>
                            <Image src={banner} alt="Banner" />
                            <Button
                                onClick={() => connect()}
                                marginY={"100px"}
                                disabled={connecting}
                            >
                                {connecting ? "Connecting" : "Connect"}
                            </Button>
                        </Stack>
                    </Box>
                )}
                <GraphQLProvider>
                    <Transfers dappAddress={dappAddress} />
                </GraphQLProvider>
            </SimpleGrid>
        </>
    );
};

export default App;
