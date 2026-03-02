import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { LiveContextProvider } from "@/hooks/useLiveContext";
import { router } from "@/router";

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 30_000,
			retry: 1,
			refetchOnWindowFocus: true,
		},
	},
});

export function App() {
	return (
		<QueryClientProvider client={queryClient}>
			<LiveContextProvider>
				<RouterProvider router={router} />
			</LiveContextProvider>
		</QueryClientProvider>
	);
}
