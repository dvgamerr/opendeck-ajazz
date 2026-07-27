export type DeviceInfo = {
	id: string;
	name: string;
	rows: number;
	columns: number;
	encoders: number;
	type: number;
	startup_image?: {
		width: number;
		height: number;
	};
};
