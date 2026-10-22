<script lang="ts">
	import FileIcon from '@lucide/svelte/icons/file';
	import HouseIcon from '@lucide/svelte/icons/house';
	import LayoutDashboardIcon from '@lucide/svelte/icons/layout-dashboard';
	import TableIcon from '@lucide/svelte/icons/table';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import ListIcon from '@lucide/svelte/icons/list';
	import SearchIcon from '@lucide/svelte/icons/search';
	import ChartColumnIcon from '@lucide/svelte/icons/chart-column';
	import ChartPieIcon from '@lucide/svelte/icons/chart-pie';
	import UsersIcon from '@lucide/svelte/icons/users';
	import UserIcon from '@lucide/svelte/icons/user';
	import BuildingIcon from '@lucide/svelte/icons/building';
	import ShoppingCartIcon from '@lucide/svelte/icons/shopping-cart';
	import PackageIcon from '@lucide/svelte/icons/package';
	import TruckIcon from '@lucide/svelte/icons/truck';
	import ReceiptIcon from '@lucide/svelte/icons/receipt';
	import WalletIcon from '@lucide/svelte/icons/wallet';
	import CreditCardIcon from '@lucide/svelte/icons/credit-card';
	import CalendarIcon from '@lucide/svelte/icons/calendar';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import InboxIcon from '@lucide/svelte/icons/inbox';
	import MailIcon from '@lucide/svelte/icons/mail';
	import PhoneIcon from '@lucide/svelte/icons/phone';
	import ClipboardListIcon from '@lucide/svelte/icons/clipboard-list';
	import SquareCheckIcon from '@lucide/svelte/icons/square-check';
	import BriefcaseIcon from '@lucide/svelte/icons/briefcase';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import FolderIcon from '@lucide/svelte/icons/folder';
	import TagIcon from '@lucide/svelte/icons/tag';
	import StarIcon from '@lucide/svelte/icons/star';
	import HeartIcon from '@lucide/svelte/icons/heart';
	import FlagIcon from '@lucide/svelte/icons/flag';
	import MapPinIcon from '@lucide/svelte/icons/map-pin';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import PrinterIcon from '@lucide/svelte/icons/printer';
	import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
	import ZapIcon from '@lucide/svelte/icons/zap';

	/** An icon from `ICONS` by name; unknown names show a page. */
	let { name, ...rest }: { name: string; class?: string; 'data-icon'?: string } = $props();

	const icons: Record<string, typeof FileIcon> = {
		'file': FileIcon,
		'house': HouseIcon,
		'layout-dashboard': LayoutDashboardIcon,
		'table': TableIcon,
		'file-text': FileTextIcon,
		'list': ListIcon,
		'search': SearchIcon,
		'chart-column': ChartColumnIcon,
		'chart-pie': ChartPieIcon,
		'users': UsersIcon,
		'user': UserIcon,
		'building': BuildingIcon,
		'shopping-cart': ShoppingCartIcon,
		'package': PackageIcon,
		'truck': TruckIcon,
		'receipt': ReceiptIcon,
		'wallet': WalletIcon,
		'credit-card': CreditCardIcon,
		'calendar': CalendarIcon,
		'clock': ClockIcon,
		'inbox': InboxIcon,
		'mail': MailIcon,
		'phone': PhoneIcon,
		'clipboard-list': ClipboardListIcon,
		'square-check': SquareCheckIcon,
		'briefcase': BriefcaseIcon,
		'book-open': BookOpenIcon,
		'folder': FolderIcon,
		'tag': TagIcon,
		'star': StarIcon,
		'heart': HeartIcon,
		'flag': FlagIcon,
		'map-pin': MapPinIcon,
		'settings': SettingsIcon,
		'plus': PlusIcon,
		'printer': PrinterIcon,
		'refresh-cw': RefreshCwIcon,
		'zap': ZapIcon
	};
	const Icon = $derived(icons[name] ?? FileIcon);
</script>

<Icon {...rest} />
