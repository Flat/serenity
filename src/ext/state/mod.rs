use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::default::Default;
use ::model::*;

/// Known state composed from received events.
#[derive(Debug, Clone)]
pub struct State {
    pub calls: HashMap<ChannelId, Call>,
    pub groups: HashMap<ChannelId, Group>,
    /// Settings specific to a guild.
    ///
    /// This will always be empty for bot accounts.
    pub guild_settings: HashMap<Option<GuildId>, UserGuildSettings>,
    pub guilds: HashMap<GuildId, LiveGuild>,
    /// A list of notes that a user has made for individual users.
    ///
    /// This will always be empty for bot accounts.
    pub notes: HashMap<UserId, String>,
    pub presences: HashMap<UserId, Presence>,
    pub private_channels: HashMap<ChannelId, PrivateChannel>,
    pub relationships: HashMap<UserId, Relationship>,
    /// Account-specific settings for a user account.
    pub settings: Option<UserSettings>,
    pub unavailable_guilds: Vec<GuildId>,
    pub user: CurrentUser,
}

impl State {
    pub fn unknown_members(&self) -> u64 {
        let mut total = 0;

        for guild in self.guilds.values() {
            let members = guild.members.len() as u64;

            if guild.member_count > members {
                total += guild.member_count - members;
            } else if guild.member_count < members {
                warn!("Inconsistent member count for {:?}: {} < {}",
                      guild.id,
                      guild.member_count,
                      members);
            }
        }

        total
    }

    pub fn all_private_channels(&self) -> Vec<ChannelId> {
        self.groups
            .keys()
            .cloned()
            .chain(self.private_channels.keys().cloned())
            .collect()
    }

    pub fn all_guilds(&self) -> Vec<GuildId> {
        self.guilds
            .values()
            .map(|s| s.id)
            .chain(self.unavailable_guilds.iter().cloned())
            .collect()
    }

    #[doc(hidden)]
    pub fn __download_members(&mut self) -> Vec<GuildId> {
        self.guilds
            .values_mut()
            .filter(|guild| guild.large)
            .map(|ref mut guild| {
                guild.members.clear();

                guild.id
            })
            .collect::<Vec<_>>()
    }

    pub fn find_call<C: Into<ChannelId>>(&self, group_id: C) -> Option<&Call> {
        self.calls.get(&group_id.into())
    }

    pub fn find_channel<C: Into<ChannelId>>(&self, id: C) -> Option<Channel> {
        let id = id.into();

        for guild in self.guilds.values() {
            for channel in guild.channels.values() {
                if channel.id == id {
                    return Some(Channel::Public(channel.clone()));
                }
            }
        }

        None
    }

    pub fn find_guild<G: Into<GuildId>>(&self, id: G) -> Option<&LiveGuild> {
        self.guilds.get(&id.into())
    }

    pub fn find_group<C: Into<ChannelId>>(&self, id: C) -> Option<&Group> {
        self.groups.get(&id.into())
    }

    pub fn find_member<G, U>(&self, guild_id: G, user_id: U)
        -> Option<&Member> where G: Into<GuildId>, U: Into<UserId> {
        self.guilds
            .get(&guild_id.into())
            .map(|guild| {
                guild.members.get(&user_id.into())
            }).and_then(|x| match x {
                Some(x) => Some(x),
                _ => None,
            })
    }

    pub fn find_role<G, R>(&self, guild_id: G, role_id: R) -> Option<&Role>
        where G: Into<GuildId>, R: Into<RoleId> {
        if let Some(guild) = self.guilds.get(&guild_id.into()) {
            guild.roles.get(&role_id.into())
        } else {
            None
        }
    }

    /// Update the state according to the changes described in the given event.
    #[allow(cyclomatic_complexity)]
    #[allow(unneeded_field_pattern)]
    pub fn update(&mut self, event: &Event) {
        match *event {
            Event::CallCreate(ref event) => {
                self.update_with_call_create(event);
            },
            Event::CallUpdate(ref event) => {
                self.update_with_call_update(event);
            },
            Event::CallDelete(ref event) => {
                self.update_with_call_delete(event);
            },
            Event::ChannelCreate(ref event) => {
                self.update_with_channel_create(event);
            },
            Event::ChannelDelete(ref event) => {
                self.update_with_channel_delete(event);
            },
            Event::ChannelPinsUpdate(ref event) => {
                self.update_with_channel_pins_update(event);
            },
            Event::ChannelRecipientAdd(ref event) => {
                self.update_with_channel_recipient_add(event);
            },
            Event::ChannelRecipientRemove(ref event) => {
                self.update_with_channel_recipient_remove(event);
            },
            Event::ChannelUpdate(ref event) => {
                self.update_with_channel_update(event);
            },
            Event::GuildCreate(ref event) => {
                self.update_with_guild_create(event);
            },
            Event::GuildDelete(ref event) => {
                self.update_with_guild_delete(event);
            },
            Event::GuildEmojisUpdate(ref event) => {
                self.update_with_guild_emojis_update(event);
            },
            Event::GuildMemberAdd(ref event) => {
                self.update_with_guild_member_add(event);
            },
            Event::GuildMemberRemove(ref event) => {
                self.update_with_guild_member_remove(event);
            },
            Event::GuildMemberUpdate(ref event) => {
                self.update_with_guild_member_update(event);
            },
            Event::GuildMembersChunk(ref event) => {
                self.update_with_guild_members_chunk(event);
            },
            Event::GuildRoleCreate(ref event) => {
                self.update_with_guild_role_create(event);
            },
            Event::GuildRoleDelete(ref event) => {
                self.update_with_guild_role_delete(event);
            },
            Event::GuildRoleUpdate(ref event) => {
                self.update_with_guild_role_update(event);
            },
            Event::GuildSync(ref event) => {
                self.update_with_guild_sync(event);
            },
            Event::GuildUnavailable(ref event) => {
                self.update_with_guild_unavailable(event);
            },
            Event::GuildUpdate(ref event) => {
                self.update_with_guild_update(event);
            },
            Event::PresencesReplace(ref event) => {
                self.update_with_presences_replace(event);
            },
            Event::PresenceUpdate(ref event) => {
                self.update_with_presence_update(event);
            },
            Event::Ready(ref event) => {
                self.update_with_ready(event);
            },
            Event::RelationshipAdd(ref event) => {
                self.update_with_relationship_add(event);
            },
            Event::RelationshipRemove(ref event) => {
                self.update_with_relationship_remove(event);
            },
            Event::UserGuildSettingsUpdate(ref event) => {
                self.update_with_user_guild_settings_update(event);
            },
            Event::UserNoteUpdate(ref event) => {
                self.update_with_user_note_update(event);
            },
            Event::UserSettingsUpdate(ref event) => {
                self.update_with_user_settings_update(event);
            },
            Event::UserUpdate(ref event) => {
                self.update_with_user_update(event);
            },
            Event::VoiceStateUpdate(ref event) => {
                self.update_with_voice_state_update(event);
            },
            _ => {},
        }
    }

    pub fn update_with_call_create(&mut self, event: &CallCreateEvent) {
        match self.calls.entry(event.call.channel_id) {
            Entry::Vacant(e) => {
                e.insert(event.call.clone());
            },
            Entry::Occupied(mut e) => {
                e.get_mut().clone_from(&event.call);
            },
        }
    }

    pub fn update_with_call_delete(&mut self, event: &CallDeleteEvent) {
        self.calls.remove(&event.channel_id);
    }

    pub fn update_with_call_update(&mut self, event: &CallUpdateEvent) {
        self.calls
            .get_mut(&event.channel_id)
            .map(|call| {
                call.region.clone_from(&event.region);
                call.ringing.clone_from(&event.ringing);
            });
    }

    pub fn update_with_channel_create(&mut self, event: &ChannelCreateEvent) {
        match event.channel {
            Channel::Group(ref group) => {
                self.groups.insert(group.channel_id, group.clone());
            },
            Channel::Private(ref channel) => {
                self.private_channels.insert(channel.id, channel.clone());
            },
            Channel::Public(ref channel) => {
                self.guilds
                    .get_mut(&channel.guild_id)
                    .map(|guild| guild.channels.insert(channel.id,
                                                       channel.clone()));
            },
        }
    }

    pub fn update_with_channel_delete(&mut self, event: &ChannelDeleteEvent) {
        match event.channel {
            Channel::Group(ref group) => {
                self.groups.remove(&group.channel_id);
            },
            Channel::Private(ref channel) => {
                self.private_channels.remove(&channel.id);
            },
            Channel::Public(ref channel) => {
                self.guilds
                    .get_mut(&channel.guild_id)
                    .map(|guild| guild.channels.remove(&channel.id));
            },
        }
    }

    pub fn update_with_channel_pins_update(&mut self,
                                           event: &ChannelPinsUpdateEvent) {
        if let Some(channel) = self.private_channels.get_mut(&event.channel_id) {
            channel.last_pin_timestamp = event.last_pin_timestamp.clone();

            return;
        }

        if let Some(group) = self.groups.get_mut(&event.channel_id) {
            group.last_pin_timestamp = event.last_pin_timestamp.clone();

            return;
        }

        // Guild searching is last because it is expensive
        // in comparison to private channel and group searching.
        for guild in self.guilds.values_mut() {
            for channel in guild.channels.values_mut() {
                if channel.id == event.channel_id {
                    channel.last_pin_timestamp = event.last_pin_timestamp.clone();

                    return;
                }
            }
        }
    }

    pub fn update_with_channel_recipient_add(&mut self,
                                             event: &ChannelRecipientAddEvent) {
        self.groups
            .get_mut(&event.channel_id)
            .map(|group| group.recipients.insert(event.user.id,
                                                 event.user.clone()));
    }

    pub fn update_with_channel_recipient_remove(&mut self,
                                                event: &ChannelRecipientRemoveEvent) {
        self.groups
            .get_mut(&event.channel_id)
            .map(|group| group.recipients.remove(&event.user.id));
    }

    pub fn update_with_channel_update(&mut self, event: &ChannelUpdateEvent) {
        match event.channel {
            Channel::Group(ref group) => {
                match self.groups.entry(group.channel_id) {
                    Entry::Vacant(e) => {
                        e.insert(group.clone());
                    },
                    Entry::Occupied(mut e) => {
                        let dest = e.get_mut();
                        if group.recipients.is_empty() {
                            // if the update omits the recipient list, preserve it
                            let recipients = ::std::mem::replace(&mut dest.recipients, HashMap::new());
                            dest.clone_from(group);
                            dest.recipients = recipients;
                        } else {
                            dest.clone_from(group);
                        }
                    },
                }
            },
            Channel::Private(ref channel) => {
                self.private_channels
                    .get_mut(&channel.id)
                    .map(|chan| chan.clone_from(channel));
            },
            Channel::Public(ref channel) => {
                self.guilds
                    .get_mut(&channel.guild_id)
                    .map(|guild| guild.channels
                        .insert(channel.id, channel.clone()));
            },
        }
    }

    pub fn update_with_guild_create(&mut self, event: &GuildCreateEvent) {
        self.guilds.insert(event.guild.id, event.guild.clone());
    }

    pub fn update_with_guild_delete(&mut self, event: &GuildDeleteEvent) {
        self.guilds.remove(&event.guild.id);
    }

    pub fn update_with_guild_emojis_update(&mut self,
                                           event: &GuildEmojisUpdateEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| guild.emojis.extend(event.emojis.clone()));
    }

    pub fn update_with_guild_member_add(&mut self,
                                        event: &GuildMemberAddEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| {
                guild.member_count += 1;
                guild.members.insert(event.member.user.id,
                                     event.member.clone());
            });
    }

    pub fn update_with_guild_member_remove(&mut self,
                                           event: &GuildMemberRemoveEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| {
                guild.member_count -= 1;
                guild.members.remove(&event.user.id);
            });
    }

    pub fn update_with_guild_member_update(&mut self,
                                           event: &GuildMemberUpdateEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| {
                let mut found = false;

                if let Some(member) = guild.members.get_mut(&event.user.id) {
                    member.nick.clone_from(&event.nick);
                    member.roles.clone_from(&event.roles);
                    member.user.clone_from(&event.user);

                    found = true;
                }

                if !found {
                    guild.members.insert(event.user.id, Member {
                        deaf: false,
                        joined_at: String::default(),
                        mute: false,
                        nick: event.nick.clone(),
                        roles: event.roles.clone(),
                        user: event.user.clone(),
                    });
                }
            });
    }

    pub fn update_with_guild_members_chunk(&mut self,
                                           event: &GuildMembersChunkEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| guild.members.extend(event.members.clone()));
    }

    pub fn update_with_guild_role_create(&mut self,
                                         event: &GuildRoleCreateEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| guild.roles.insert(event.role.id, event.role.clone()));
    }

    pub fn update_with_guild_role_delete(&mut self,
                                         event: &GuildRoleDeleteEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| guild.roles.remove(&event.role_id));
    }

    pub fn update_with_guild_role_update(&mut self,
                                         event: &GuildRoleUpdateEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| guild.roles
                .get_mut(&event.role.id)
                .map(|role| role.clone_from(&event.role)));
    }

    pub fn update_with_guild_sync(&mut self, event: &GuildSyncEvent) {
        self.guilds
            .get_mut(&event.guild_id)
            .map(|guild| {
                guild.large = event.large;
                guild.members.clone_from(&event.members);
                guild.presences.clone_from(&event.presences);
            });
    }

    pub fn update_with_guild_unavailable(&mut self,
                                         event: &GuildUnavailableEvent) {
        if !self.unavailable_guilds.contains(&event.guild_id) {
            self.unavailable_guilds.push(event.guild_id);
        }
    }

    pub fn update_with_guild_update(&mut self, event: &GuildUpdateEvent) {
        self.guilds
            .get_mut(&event.guild.id)
            .map(|guild| {
                // todo: embed
                guild.afk_timeout = event.guild.afk_timeout;
                guild.afk_channel_id.clone_from(&event.guild.afk_channel_id);
                guild.icon.clone_from(&event.guild.icon);
                guild.name.clone_from(&event.guild.name);
                guild.owner_id.clone_from(&event.guild.owner_id);
                guild.region.clone_from(&event.guild.region);
                guild.roles.clone_from(&event.guild.roles);
                guild.verification_level = event.guild.verification_level;
            });
    }

    pub fn update_with_presences_replace(&mut self, event: &PresencesReplaceEvent) {
        self.presences.clone_from(&{
            let mut p = HashMap::default();

            for presence in &event.presences {
                p.insert(presence.user_id, presence.clone());
            }

            p
        });
    }

    pub fn update_with_presence_update(&mut self, event: &PresenceUpdateEvent) {
        if let Some(guild_id) = event.guild_id {
            if let Some(guild) = self.guilds.get_mut(&guild_id) {
                // If the user was modified, update the member list
                if let Some(user) = event.presence.user.as_ref() {
                    guild.members
                        .get_mut(&user.id)
                        .map(|member| member.user.clone_from(user));
                }

                update_presence(&mut guild.presences, &event.presence);
            }
        }
    }

    pub fn update_with_ready(&mut self, ready: &ReadyEvent) {
        let ready = ready.ready.clone();

        for guild in ready.guilds {
            match guild {
                PossibleGuild::Offline(guild_id) => {
                    self.unavailable_guilds.push(guild_id);
                }
                PossibleGuild::Online(guild) => {
                    self.guilds.insert(guild.id, guild);
                },
            }
        }

        self.unavailable_guilds.sort();
        self.unavailable_guilds.dedup();

        // The private channels sent in the READY contains both the actual
        // private channels, and the groups.
        for (channel_id, channel) in ready.private_channels {
            match channel {
                Channel::Group(group) => {
                    self.groups.insert(channel_id, group);
                },
                Channel::Private(channel) => {
                    self.private_channels.insert(channel_id, channel);
                },
                Channel::Public(_) => {},
            }
        }

        for guild in ready.user_guild_settings.unwrap_or_default() {
            self.guild_settings.insert(guild.guild_id, guild);
        }

        for (user_id, presence) in ready.presences {
            self.presences.insert(user_id, presence);
        }

        for (user_id, relationship) in ready.relationships {
            self.relationships.insert(user_id, relationship);
        }

        self.notes.extend(ready.notes);

        self.settings = ready.user_settings;
        self.user = ready.user;
    }

    pub fn update_with_relationship_add(&mut self, event: &RelationshipAddEvent) {
        self.relationships.insert(event.relationship.id,
                                  event.relationship.clone());
    }

    pub fn update_with_relationship_remove(&mut self,
                                           event: &RelationshipRemoveEvent) {
        self.relationships.remove(&event.user_id);
    }

    pub fn update_with_user_guild_settings_update(&mut self,
                                                  event: &UserGuildSettingsUpdateEvent) {
        self.guild_settings
            .get_mut(&event.settings.guild_id)
            .map(|guild_setting| guild_setting.clone_from(&event.settings));
    }

    pub fn update_with_user_note_update(&mut self,
                                        event: &UserNoteUpdateEvent) {
        if event.note.is_empty() {
            self.notes.remove(&event.user_id);
        } else {
            self.notes.insert(event.user_id, event.note.clone());
        }
    }

    pub fn update_with_user_settings_update(&mut self,
                                            event: &UserSettingsUpdateEvent) {
        self.settings
            .as_mut()
            .map(|settings| {
                opt_modify(&mut settings.enable_tts_command, &event.enable_tts_command);
                opt_modify(&mut settings.inline_attachment_media, &event.inline_attachment_media);
                opt_modify(&mut settings.inline_embed_media, &event.inline_embed_media);
                opt_modify(&mut settings.locale, &event.locale);
                opt_modify(&mut settings.message_display_compact, &event.message_display_compact);
                opt_modify(&mut settings.render_embeds, &event.render_embeds);
                opt_modify(&mut settings.show_current_game, &event.show_current_game);
                opt_modify(&mut settings.theme, &event.theme);
                opt_modify(&mut settings.convert_emoticons, &event.convert_emoticons);
                opt_modify(&mut settings.friend_source_flags, &event.friend_source_flags);
            });
    }

    pub fn update_with_user_update(&mut self, event: &UserUpdateEvent) {
        self.user = event.current_user.clone();
    }

    pub fn update_with_voice_state_update(&mut self,
                                          event: &VoiceStateUpdateEvent) {
        if let Some(guild_id) = event.guild_id {
            if let Some(guild) = self.guilds.get_mut(&guild_id) {
                if !event.voice_state.channel_id.is_some() {
                    // Remove the user from the voice state list
                    guild.voice_states.remove(&event.voice_state.user_id);
                } else {
                    // Update or add to the voice state list
                    {
                        let finding = guild.voice_states
                            .get_mut(&event.voice_state.user_id);

                        if let Some(srv_state) = finding {
                            srv_state.clone_from(&event.voice_state);

                            return;
                        }
                    }

                    guild.voice_states.insert(event.voice_state.user_id,
                                              event.voice_state.clone());
                }
            }

            return;
        }

        if let Some(channel) = event.voice_state.channel_id {
            // channel id available, insert voice state
            if let Some(call) = self.calls.get_mut(&channel) {
                {
                    let finding = call.voice_states
                        .get_mut(&event.voice_state.user_id);

                    if let Some(grp_state) = finding {
                        grp_state.clone_from(&event.voice_state);

                        return;
                    }
                }

                call.voice_states.insert(event.voice_state.user_id,
                                         event.voice_state.clone());
            }
        } else {
            // delete this user from any group call containing them
            for call in self.calls.values_mut() {
                call.voice_states.remove(&event.voice_state.user_id);
            }
        }
    }
}

impl Default for State {
    fn default() -> State {
        State {
            calls: HashMap::default(),
            groups: HashMap::default(),
            guild_settings: HashMap::default(),
            guilds: HashMap::default(),
            notes: HashMap::default(),
            presences: HashMap::default(),
            private_channels: HashMap::default(),
            relationships: HashMap::default(),
            settings: None,
            unavailable_guilds: Vec::default(),
            user: CurrentUser {
                avatar: None,
                bot: false,
                discriminator: 0,
                email: None,
                id: UserId(0),
                mfa_enabled: false,
                mobile: None,
                name: String::default(),
                verified: false,
            }
        }
    }
}

fn update_presence(presences: &mut HashMap<UserId, Presence>,
                   presence: &Presence) {
    if presence.status == OnlineStatus::Offline {
        // Remove the user from the presence list
        presences.remove(&presence.user_id);
    } else {
        // Update or add to the presence list
        if let Some(ref mut guild_presence) = presences.get(&presence.user_id) {
            if presence.user.is_none() {
                guild_presence.clone_from(&presence);
            }

            return;
        }
        presences.insert(presence.user_id, presence.clone());
    }
}

/// A reference to a private channel, public channel, or group.
#[derive(Debug, Clone, Copy)]
pub enum ChannelRef<'a> {
    /// A private channel
    Private(&'a PrivateChannel),
    /// A group channel
    Group(&'a Group),
    /// A public channel and its guild
    Public(&'a LiveGuild, &'a PublicChannel),
}

fn opt_modify<T: Clone>(dest: &mut T, src: &Option<T>) {
    if let Some(val) = src.as_ref() {
        dest.clone_from(val);
    }
}
